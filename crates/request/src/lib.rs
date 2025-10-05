//! Request context component for the Model Context Protocol (MCP)
//!
//! This component provides request parsing and context management for MCP handlers.
//! It is responsible for:
//! - Parsing JSON-RPC requests from input streams
//! - Validating request structure and extracting typed components
//! - Managing request-scoped context for middleware
//! - Providing lazy evaluation of method and parameter variants
//!
//! ## Request Lifecycle
//!
//! 1. Input stream is buffered and parsed as JSON-RPC
//! 2. Request resource is created with internal JSON representation
//! 3. Methods like `id()`, `method()`, and `params()` parse on-demand
//! 4. Middleware can attach context via `get()`/`set()` operations
//! 5. `to_json()` consumes the resource to prevent memory duplication
//!
//! ## Context Management
//!
//! Each request maintains an in-memory context map for middleware to attach
//! metadata during request processing. This context is scoped to the request
//! lifetime and is not persisted across requests.

mod bindings {
    wit_bindgen::generate!({
        world: "request",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp::request::{
    CallToolParams, CompletionParams, GetPromptParams, GuestRequest, InitializeParams,
    Method, Params,
};
use bindings::wasi::io::streams::{InputStream, StreamError};
use bindings::wasmcp::mcp::error::{Error as McpError, ErrorCode};
use bindings::wasmcp::mcp::types::{
    ClientCapabilities, Id, Implementation, ProtocolVersion, ServerCapabilities,
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
    fn from_http_stream(
        input: &InputStream,
    ) -> Result<bindings::exports::wasmcp::mcp::request::Request, StreamError> {
        // Read entire HTTP request body until EOF.
        // HTTP bodies are fully buffered, so use non-blocking read().
        // Read in 64KB chunks until we get a short read (indicating end of body).

        let mut buffer = Vec::new();
        let chunk_size = 65536;

        loop {
            match input.read(chunk_size) {
                Ok(chunk) => {
                    let was_full_read = chunk.len() == chunk_size as usize;
                    buffer.extend_from_slice(&chunk);

                    // If we read less than requested, we've reached the end of the HTTP body
                    if !was_full_read {
                        break;
                    }
                }
                Err(e) => return Err(e),
            }
        }

        // Parse the complete request body as JSON-RPC
        Self::parse_message(&buffer)
    }

    fn from_stdio_stream(
        input: &InputStream,
    ) -> Result<bindings::exports::wasmcp::mcp::request::Request, StreamError> {
        // Read newline-delimited JSON-RPC message from stdio.
        // Stdio transport provides newline-delimited messages (\n or \r\n).
        // Reads until newline found, supports persistent connections with multiple messages.

        let mut buffer = Vec::new();
        let chunk_size = 65536;

        loop {
            match input.blocking_read(chunk_size) {
                Ok(chunk) => {
                    if chunk.is_empty() {
                        // EOF without newline - unexpected for stdio
                        return Err(StreamError::Closed);
                    }

                    buffer.extend_from_slice(&chunk);

                    // Check for newline delimiter
                    if let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                        // Extract message up to (not including) newline
                        let mut message_bytes = buffer[..newline_pos].to_vec();

                        // Strip trailing \r if present (CRLF line endings)
                        if message_bytes.last() == Some(&b'\r') {
                            message_bytes.pop();
                        }

                        // Parse and return the complete message
                        return Self::parse_message(&message_bytes);
                    }

                    // No newline yet - continue reading
                }
                Err(e) => return Err(e),
            }
        }
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

    fn params(&self) -> Result<Params, McpError> {
        let method_str = self
            .parsed
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("");

        let params_value = self.parsed.get("params");

        match method_str {
            "initialize" => {
                if let Some(params) = params_value {
                    match parse_initialize_params(params) {
                        Ok(init_params) => Ok(Params::Initialize(init_params)),
                        Err(e) => Err(McpError {
                            code: ErrorCode::InvalidParams,
                            message: format!("Invalid initialize params: {}", e),
                            data: None,
                        }),
                    }
                } else {
                    Err(McpError {
                        code: ErrorCode::InvalidParams,
                        message: "Missing params for initialize method".to_string(),
                        data: None,
                    })
                }
            }
            "tools/list" => {
                let cursor = params_value
                    .and_then(|p| p.get("cursor"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                Ok(Params::ToolsList(cursor))
            }
            "tools/call" => {
                if let Some(params) = params_value {
                    let name = params
                        .get("name")
                        .and_then(|n| n.as_str())
                        .ok_or_else(|| McpError {
                            code: ErrorCode::InvalidParams,
                            message: "Missing 'name' field in tools/call params".to_string(),
                            data: None,
                        })?
                        .to_string();
                    let arguments = params.get("arguments").map(|a| a.to_string());

                    Ok(Params::ToolsCall(CallToolParams { name, arguments }))
                } else {
                    Err(McpError {
                        code: ErrorCode::InvalidParams,
                        message: "Missing params for tools/call method".to_string(),
                        data: None,
                    })
                }
            }
            "resources/list" => {
                let cursor = params_value
                    .and_then(|p| p.get("cursor"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                Ok(Params::ResourcesList(cursor))
            }
            "resources/read" => {
                let uri = params_value
                    .and_then(|p| p.get("uri"))
                    .and_then(|u| u.as_str())
                    .ok_or_else(|| McpError {
                        code: ErrorCode::InvalidParams,
                        message: "Missing 'uri' field in resources/read params".to_string(),
                        data: None,
                    })?
                    .to_string();
                Ok(Params::ResourcesRead(uri))
            }
            "resources/templates/list" => {
                let cursor = params_value
                    .and_then(|p| p.get("cursor"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                Ok(Params::ResourcesTemplatesList(cursor))
            }
            "prompts/list" => {
                let cursor = params_value
                    .and_then(|p| p.get("cursor"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                Ok(Params::PromptsList(cursor))
            }
            "prompts/get" => {
                if let Some(params) = params_value {
                    let name = params
                        .get("name")
                        .and_then(|n| n.as_str())
                        .ok_or_else(|| McpError {
                            code: ErrorCode::InvalidParams,
                            message: "Missing 'name' field in prompts/get params".to_string(),
                            data: None,
                        })?
                        .to_string();
                    let arguments = params
                        .get("arguments")
                        .map(|v| v.to_string());

                    Ok(Params::PromptsGet(GetPromptParams { name, arguments }))
                } else {
                    Err(McpError {
                        code: ErrorCode::InvalidParams,
                        message: "Missing params for prompts/get method".to_string(),
                        data: None,
                    })
                }
            }
            "completion/complete" => {
                if let Some(params) = params_value {
                    let completion_params = parse_completion_params(params)?;
                    Ok(Params::CompletionComplete(completion_params))
                } else {
                    Err(McpError {
                        code: ErrorCode::InvalidParams,
                        message: "Missing params for completion/complete method".to_string(),
                        data: None,
                    })
                }
            }
            _ => Err(McpError {
                code: ErrorCode::MethodNotFound,
                message: format!("Unknown method: {}", method_str),
                data: None,
            }),
        }
    }

    fn get(&self, key: String) -> Result<Option<Vec<u8>>, ()> {
        Ok(self.context
            .read()
            .ok()
            .and_then(|ctx| ctx.get(&key).cloned()))
    }

    fn set(&self, key: String, value: Vec<u8>) -> Result<(), ()> {
        if let Ok(mut ctx) = self.context.write() {
            ctx.insert(key, value);
        }
        Ok(())
    }

    fn needs(&self, capabilities: ServerCapabilities) -> bool {
        let method = self.method();

        // Handle initialize separately
        if matches!(method, Method::Initialize) {
            // Register capabilities during initialize and forward
            if let Ok(mut caps) = self.capabilities.write() {
                if let Some(existing) = *caps {
                    *caps = Some(existing | capabilities);
                } else {
                    *caps = Some(capabilities);
                }
            }
            return false; // Forward to next handler
        }

        // Check if method matches capabilities
        match method {
            Method::ToolsList | Method::ToolsCall => capabilities.contains(ServerCapabilities::TOOLS),
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

impl Request {
    fn parse_message(
        bytes: &[u8],
    ) -> Result<bindings::exports::wasmcp::mcp::request::Request, StreamError> {
        // Parse JSON-RPC request
        let parsed: Value = serde_json::from_slice(bytes).map_err(|_| StreamError::Closed)?;

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
fn parse_completion_params(params: &Value) -> Result<CompletionParams, McpError> {
    use bindings::exports::wasmcp::mcp::request::{CompletionArgument, CompletionRef};

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
    let ref_ = CompletionRef::Prompt(bindings::exports::wasmcp::mcp::request::CompletionPromptReference {
        name: String::new(),
        title: None,
    });

    Ok(CompletionParams {
        argument,
        ref_,
        context: None,
    })
}

impl bindings::exports::wasmcp::mcp::request::Guest for Component {
    type Request = Request;
}

bindings::export!(Component with_types_in bindings);
