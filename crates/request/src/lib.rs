//! Request context component for the Model Context Protocol (MCP)
//!
//! This component provides request parsing and context management for MCP handlers.
//! It is responsible for:
//! - Parsing JSON-RPC requests from byte arrays
//! - Validating request structure and extracting typed components
//! - Managing request-scoped context for middleware
//! - Tracking and checking capability requirements
//! - Providing lazy evaluation of method and parameter variants

mod bindings {
    wit_bindgen::generate!({
        world: "request",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp::request::{
    ArgParams, CompleteParams, GuestRequest, InitializeParams,
    Method, Params, CompletionArgument, CompletionRef,
    CompletionPromptReference, CompletionContext,
};
use bindings::wasi::io::streams::StreamError;
use bindings::wasmcp::mcp::protocol::{
    Error as McpError, ErrorCode, Id, ClientCapabilities,
    Implementation, ProtocolVersion, ServerCapabilities,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;

struct Component;

struct Request {
    parsed: Value,
    context: RwLock<HashMap<String, Vec<u8>>>,
    capabilities: RwLock<Option<ServerCapabilities>>,
}

// JSON-RPC request structure for validation
#[derive(Deserialize, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

impl GuestRequest for Request {
    fn from_bytes(bytes: Vec<u8>) -> Result<bindings::exports::wasmcp::mcp::request::Request, StreamError> {
        // Parse JSON-RPC request
        let parsed: Value = serde_json::from_slice(&bytes).map_err(|_| StreamError::Closed)?;

        // Validate JSON-RPC structure
        if !parsed.is_object() {
            return Err(StreamError::Closed);
        }

        // Create request with parsed JSON
        Ok(bindings::exports::wasmcp::mcp::request::Request::new(
            Request {
                parsed,
                context: RwLock::new(HashMap::new()),
                capabilities: RwLock::new(None),
            },
        ))
    }

    fn id(&self) -> Id {
        self.parsed
            .get("id")
            .and_then(|id| {
                if let Some(num) = id.as_i64() {
                    Some(Id::Number(num))
                } else {
                    id.as_str().map(|s| Id::String(s.to_string()))
                }
            })
            .unwrap_or(Id::String(String::new()))
    }

    fn method(&self) -> Method {
        let method_str = self
            .parsed
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("");

        match method_str {
            "initialize" => Method::Initialize,
            "tools/list" => Method::ToolsList,
            "tools/call" => Method::ToolsCall,
            "resources/list" => Method::ResourcesList,
            "resources/read" => Method::ResourcesRead,
            "resources/templates/list" => Method::ResourcesTemplatesList,
            "prompts/list" => Method::PromptsList,
            "prompts/get" => Method::PromptsGet,
            "completion/complete" => Method::CompletionComplete,
            "notifications/initialized" => Method::NotificationsInitialized,
            "ping" => Method::Ping,
            _ => Method::Initialize, // Default to initialize for unknown methods
        }
    }

    fn params(&self) -> Option<Params> {
        let method_str = self
            .parsed
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("");

        let params_value = self.parsed.get("params");

        match method_str {
            "initialize" => {
                if let Some(params) = params_value {
                    parse_initialize_params(params)
                        .ok()
                        .map(Params::Initialize)
                } else {
                    None
                }
            }
            "tools/list" => {
                let cursor = params_value
                    .and_then(|p| p.get("cursor"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());
                Some(Params::ToolsList(cursor))
            }
            "tools/call" => {
                params_value.and_then(|params| {
                    let name = params.get("name")?.as_str()?.to_string();
                    let arguments = params.get("arguments").and_then(|a| a.as_str()).map(|s| s.to_string());
                    Some(Params::ToolsCall(ArgParams { name, arguments }))
                })
            }
            "resources/list" => {
                let cursor = params_value
                    .and_then(|p| p.get("cursor"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());
                Some(Params::ResourcesList(cursor))
            }
            "resources/read" => {
                params_value.and_then(|p| {
                    let uri = p.get("uri")?.as_str()?.to_string();
                    Some(Params::ResourcesRead(uri))
                })
            }
            "resources/templates/list" => {
                let cursor = params_value
                    .and_then(|p| p.get("cursor"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());
                Some(Params::ResourcesTemplatesList(cursor))
            }
            "prompts/list" => {
                let cursor = params_value
                    .and_then(|p| p.get("cursor"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());
                Some(Params::PromptsList(cursor))
            }
            "prompts/get" => {
                params_value.and_then(|params| {
                    let name = params.get("name")?.as_str()?.to_string();
                    let arguments = params.get("arguments").and_then(|a| a.as_str()).map(|s| s.to_string());
                    Some(Params::PromptsGet(ArgParams { name, arguments }))
                })
            }
            "completion/complete" => {
                params_value.and_then(|params| {
                    parse_completion_params(params).ok().map(Params::CompletionComplete)
                })
            }
            _ => None,
        }
    }

    fn get(&self, key: String) -> Option<String> {
        self.context
            .read()
            .ok()
            .and_then(|ctx| ctx.get(&key).cloned())
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }

    fn set(&self, key: String, value: String) {
        if let Ok(mut ctx) = self.context.write() {
            ctx.insert(key, value.into_bytes());
        }
    }

    fn needs(&self, capabilities: ServerCapabilities) -> bool {
        let method = self.method();

        // Handle initialize separately - register capabilities and forward
        if matches!(method, Method::Initialize) {
            if let Ok(mut caps) = self.capabilities.write() {
                if let Some(existing) = *caps {
                    *caps = Some(existing | capabilities);
                } else {
                    *caps = Some(capabilities);
                }
            }
            return false; // Always forward initialize to next handler
        }

        // Check if method matches capabilities
        match method {
            Method::ToolsList | Method::ToolsCall => {
                capabilities.contains(ServerCapabilities::TOOLS)
            }
            Method::ResourcesList | Method::ResourcesRead | Method::ResourcesTemplatesList => {
                capabilities.contains(ServerCapabilities::RESOURCES)
            }
            Method::PromptsList | Method::PromptsGet => {
                capabilities.contains(ServerCapabilities::PROMPTS)
            }
            Method::CompletionComplete => {
                capabilities.contains(ServerCapabilities::COMPLETIONS)
            }
            _ => false,
        }
    }

    fn get_capabilities(&self) -> Result<Option<ServerCapabilities>, ()> {
        self.capabilities.read().map(|caps| *caps).map_err(|_| ())
    }
}

// Helper function to parse initialize params
fn parse_initialize_params(params: &Value) -> Result<InitializeParams, String> {
    let protocol_version = params
        .get("protocolVersion")
        .and_then(|v| v.as_str())
        .map(|s| match s {
            "2025-06-18" => ProtocolVersion::V20250618,
            "2025-03-26" => ProtocolVersion::V20250326,
            "2024-11-05" => ProtocolVersion::V20241105,
            _ => ProtocolVersion::V20250618,
        })
        .unwrap_or(ProtocolVersion::V20250618);

    let client_info = params
        .get("clientInfo")
        .and_then(|ci| {
            let name = ci.get("name")?.as_str()?.to_string();
            let version = ci.get("version")?.as_str()?.to_string();
            let title = ci.get("title").and_then(|t| t.as_str()).map(String::from);
            Some(Implementation {
                name,
                version,
                title,
            })
        })
        .ok_or_else(|| "Missing or invalid clientInfo".to_string())?;

    let capabilities_flags = params
        .get("capabilities")
        .and_then(|caps| {
            let mut flags = ClientCapabilities::empty();
            if caps.get("elicitation").is_some() {
                flags |= ClientCapabilities::ELICITATION;
            }
            if caps.get("roots").is_some() {
                flags |= ClientCapabilities::ROOTS;
            }
            if caps.get("sampling").is_some() {
                flags |= ClientCapabilities::SAMPLING;
            }
            if caps.get("experimental").is_some() {
                flags |= ClientCapabilities::EXPERIMENTAL;
            }
            Some(flags)
        })
        .unwrap_or_else(ClientCapabilities::empty);

    Ok(InitializeParams {
        protocol_version,
        client_info,
        capabilities: capabilities_flags,
    })
}

// Helper function to parse completion params
fn parse_completion_params(params: &Value) -> Result<CompleteParams, McpError> {
    // Parse argument
    let argument = {
        let arg = params.get("argument").ok_or_else(|| McpError {
            code: ErrorCode::InvalidParams,
            message: "Missing 'argument' in completion params".to_string(),
            data: None,
        })?;
        CompletionArgument {
            name: arg.get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| McpError {
                    code: ErrorCode::InvalidParams,
                    message: "Missing 'name' in completion argument".to_string(),
                    data: None,
                })?
                .to_string(),
            value: arg.get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError {
                    code: ErrorCode::InvalidParams,
                    message: "Missing 'value' in completion argument".to_string(),
                    data: None,
                })?
                .to_string(),
        }
    };

    // Parse ref (simplified - just using empty prompt ref for now)
    let ref_ = CompletionRef::Prompt(CompletionPromptReference {
        name: String::new(),
        title: None,
    });

    // Parse optional context
    let context = params.get("context").and_then(|ctx| {
        ctx.get("arguments").and_then(|args| {
            args.as_str().map(|s| CompletionContext {
                arguments: Some(s.to_string()),
            })
        })
    });

    Ok(CompleteParams {
        argument,
        ref_,
        context,
    })
}

impl bindings::exports::wasmcp::mcp::request::Guest for Component {
    type Request = Request;
}

bindings::export!(Component with_types_in bindings);