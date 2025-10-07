//! Request context component for the Model Context Protocol (MCP)
//!
//! This component provides request parsing and context management for MCP handlers.
//! It is responsible for:
//! - Parsing JSON-RPC 2.0 requests from byte arrays
//! - Validating request structure and extracting typed components
//! - Managing request-scoped context for middleware communication
//! - Providing strongly-typed access to method and parameter variants
//!
//! # Architecture
//!
//! The request component acts as the entry point for all MCP requests,
//! converting raw JSON-RPC bytes into structured Rust types that handlers
//! can work with. It maintains a request-scoped context that middleware
//! components can use to share state during request processing.
//!
//! # Example
//!
//! ```
//! // Transport component reads bytes from network/stdio
//! let bytes = read_from_transport();
//!
//! // Parse into a Request object
//! let request = Request::from_bytes(bytes)?;
//!
//! // Access request properties
//! let id = request.id();
//! let method = request.method();
//! let params = request.params();
//! ```

mod bindings {
    wit_bindgen::generate!({
        world: "request",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp::request::{
    Cancellation, CompleteParams, CompletionArgument, CompletionContext,
    CompletionPromptReference, CompletionRef, ElicitResult, ElicitResultAction,
    ElicitResultContent, GuestRequest, InitializeParams, Method, Params, PromptsGetParams,
    RequestBorrow, ToolsCallParams,
};
use bindings::wasmcp::mcp::protocol::{
    ClientCapabilities, Id, Implementation, ProgressToken, ProtocolVersion, ServerCapabilities,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;

struct Component;

/// Internal representation of an MCP request.
///
/// This struct holds the parsed JSON-RPC request and provides
/// thread-safe access to request-scoped context.
struct Request {
    /// The parsed JSON-RPC request
    parsed: Value,
    /// Request-scoped context for middleware communication
    context: RwLock<HashMap<String, String>>,
}


impl GuestRequest for Request {
    /// Parse a JSON-RPC 2.0 request from a byte array.
    ///
    /// This method validates the JSON structure and ensures it conforms
    /// to the JSON-RPC 2.0 specification before creating a Request object.
    fn from_bytes(bytes: Vec<u8>) -> Result<bindings::exports::wasmcp::mcp::request::Request, ()> {
        // Parse as generic JSON first to keep flexibility
        let parsed: Value = serde_json::from_slice(&bytes)
            .map_err(|_| ())?;

        // Validate it's a JSON object
        if !parsed.is_object() {
            return Err(());
        }

        // Validate JSON-RPC 2.0 structure
        let jsonrpc = parsed
            .get("jsonrpc")
            .and_then(|v| v.as_str())
            .ok_or(())?;

        if jsonrpc != "2.0" {
            return Err(());
        }

        // Method field is required
        if !parsed.get("method").is_some() {
            return Err(());
        }

        // Create request with parsed JSON
        Ok(bindings::exports::wasmcp::mcp::request::Request::new(
            Request {
                parsed,
                context: RwLock::new(HashMap::new()),
            },
        ))
    }

    /// Get the request ID if present.
    ///
    /// Returns None for notification requests (which don't have IDs).
    fn id(&self) -> Option<Id> {
        self.parsed
            .get("id")
            .and_then(|id| {
                if let Some(num) = id.as_i64() {
                    Some(Id::Number(num))
                } else {
                    id.as_str().map(|s| Id::String(s.to_string()))
                }
            })
    }

    /// Get the method of this request.
    ///
    /// Returns the strongly-typed Method enum variant corresponding
    /// to the JSON-RPC method string.
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
            "notifications/cancelled" => Method::NotificationsCancelled,
            "ping" => Method::Ping,
            "elicit/result" => Method::ElicitResult,
            _ => Method::Ping, // Default to ping for unknown methods (safest no-op)
        }
    }

    /// Get the parameters of this request.
    ///
    /// Parses the JSON parameters into the appropriate strongly-typed
    /// Params variant based on the request method.
    fn params(&self) -> Params {
        let method = self.method();
        let params_value = self.parsed.get("params");

        match method {
            Method::Initialize => {
                params_value
                    .and_then(|p| parse_initialize_params(p).ok())
                    .map(Params::Initialize)
                    .unwrap_or(Params::None)
            }
            Method::ToolsList => {
                let cursor = params_value
                    .and_then(|p| p.get("cursor"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());
                Params::ToolsList(cursor)
            }
            Method::ToolsCall => {
                params_value
                    .and_then(|p| parse_tools_call_params(p).ok())
                    .map(Params::ToolsCall)
                    .unwrap_or(Params::None)
            }
            Method::ResourcesList => {
                let cursor = params_value
                    .and_then(|p| p.get("cursor"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());
                Params::ResourcesList(cursor)
            }
            Method::ResourcesRead => {
                params_value
                    .and_then(|p| p.get("uri"))
                    .and_then(|u| u.as_str())
                    .map(|s| Params::ResourcesRead(s.to_string()))
                    .unwrap_or(Params::None)
            }
            Method::ResourcesTemplatesList => {
                let cursor = params_value
                    .and_then(|p| p.get("cursor"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());
                Params::ResourcesTemplatesList(cursor)
            }
            Method::PromptsList => {
                let cursor = params_value
                    .and_then(|p| p.get("cursor"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());
                Params::PromptsList(cursor)
            }
            Method::PromptsGet => {
                params_value
                    .and_then(|p| parse_prompts_get_params(p).ok())
                    .map(Params::PromptsGet)
                    .unwrap_or(Params::None)
            }
            Method::CompletionComplete => {
                params_value
                    .and_then(|p| parse_complete_params(p).ok())
                    .map(Params::CompletionComplete)
                    .unwrap_or(Params::None)
            }
            Method::ElicitResult => {
                params_value
                    .and_then(|p| parse_elicit_result(p).ok())
                    .map(Params::ElicitResult)
                    .unwrap_or(Params::None)
            }
            Method::NotificationsCancelled => {
                params_value
                    .and_then(|p| parse_cancellation(p).ok())
                    .map(Params::Cancellation)
                    .unwrap_or(Params::None)
            }
            _ => Params::None,
        }
    }

    /// Get a progress token for this request, if any.
    ///
    /// Progress tokens allow servers to report incremental progress
    /// for long-running operations.
    fn progress_token(&self) -> Option<ProgressToken> {
        self.parsed
            .get("_meta")
            .and_then(|meta| meta.get("progressToken"))
            .and_then(|token| {
                if let Some(s) = token.as_str() {
                    Some(ProgressToken::String(s.to_string()))
                } else if let Some(n) = token.as_i64() {
                    Some(ProgressToken::Integer(n))
                } else {
                    None
                }
            })
    }

    /// Get a request-scoped context value by key.
    ///
    /// This allows middleware components to retrieve shared state
    /// that was set earlier in the request processing chain.
    fn get(&self, key: String) -> Option<String> {
        self.context
            .read()
            .ok()
            .and_then(|ctx| ctx.get(&key).cloned())
    }

    /// Set a request-scoped context value by key.
    ///
    /// This allows middleware components to share state with components
    /// later in the request processing chain.
    fn set(&self, key: String, value: String) {
        if let Ok(mut ctx) = self.context.write() {
            ctx.insert(key, value);
        }
    }
}


// === Helper Parsing Functions ===

/// Parse initialize request parameters.
fn parse_initialize_params(params: &Value) -> Result<InitializeParams, String> {
    let protocol_version = params
        .get("protocolVersion")
        .and_then(|v| v.as_str())
        .map(|s| match s {
            "2025-06-18" => ProtocolVersion::V20250618,
            "2025-03-26" => ProtocolVersion::V20250326,
            "2024-11-05" => ProtocolVersion::V20241105,
            _ => ProtocolVersion::V20250618, // Default to latest
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

    let capabilities = params
        .get("capabilities")
        .map(|caps| {
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
            flags
        })
        .unwrap_or_else(ClientCapabilities::empty);

    Ok(InitializeParams {
        capabilities,
        client_info,
        protocol_version,
    })
}

/// Parse tools/call request parameters.
fn parse_tools_call_params(params: &Value) -> Result<ToolsCallParams, String> {
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| "Missing 'name' in tools/call params".to_string())?
        .to_string();

    let arguments = params
        .get("arguments")
        .map(|a| serde_json::to_string(a).unwrap_or_else(|_| a.to_string()));

    Ok(ToolsCallParams { name, arguments })
}

/// Parse prompts/get request parameters.
fn parse_prompts_get_params(params: &Value) -> Result<PromptsGetParams, String> {
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| "Missing 'name' in prompts/get params".to_string())?
        .to_string();

    let arguments = params
        .get("arguments")
        .map(|a| serde_json::to_string(a).unwrap_or_else(|_| a.to_string()));

    Ok(PromptsGetParams { name, arguments })
}

/// Parse completion/complete request parameters.
fn parse_complete_params(params: &Value) -> Result<CompleteParams, String> {
    // Parse argument
    let argument = params
        .get("argument")
        .ok_or_else(|| "Missing 'argument' in completion params".to_string())
        .and_then(|arg| {
            let name = arg
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| "Missing 'name' in completion argument".to_string())?
                .to_string();
            let value = arg
                .get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'value' in completion argument".to_string())?
                .to_string();
            Ok(CompletionArgument { name, value })
        })?;

    // Parse ref
    let ref_ = params
        .get("ref")
        .ok_or_else(|| "Missing 'ref' in completion params".to_string())
        .and_then(|r| {
            if let Some(prompt) = r.get("prompt") {
                let name = prompt
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string();
                let title = prompt
                    .get("title")
                    .and_then(|t| t.as_str())
                    .map(String::from);
                Ok(CompletionRef::Prompt(CompletionPromptReference { name, title }))
            } else if let Some(template) = r.get("resourceTemplate") {
                let uri = template.as_str().unwrap_or("").to_string();
                Ok(CompletionRef::ResourceTemplate(uri))
            } else {
                Err("Invalid 'ref' in completion params".to_string())
            }
        })?;

    // Parse optional context
    let context = params.get("context").map(|ctx| {
        let arguments = ctx
            .get("arguments")
            .and_then(|args| args.as_str())
            .map(String::from);
        CompletionContext { arguments }
    });

    Ok(CompleteParams {
        argument,
        ref_,
        context,
    })
}

/// Parse elicit/result parameters.
fn parse_elicit_result(params: &Value) -> Result<ElicitResult, String> {
    let meta = params
        .get("meta")
        .and_then(|m| {
            if m.is_object() {
                Some(
                    m.as_object()?
                        .iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                        .collect(),
                )
            } else {
                None
            }
        });

    let action = params
        .get("action")
        .and_then(|a| a.as_str())
        .map(|s| match s {
            "accept" => ElicitResultAction::Accept,
            "decline" => ElicitResultAction::Decline,
            "cancel" => ElicitResultAction::Cancel,
            _ => ElicitResultAction::Cancel,
        })
        .unwrap_or(ElicitResultAction::Cancel);

    let content = params.get("content").and_then(|c| {
        c.as_object().map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| {
                    let content = if let Some(s) = v.as_str() {
                        ElicitResultContent::String(s.to_string())
                    } else if let Some(n) = v.as_f64() {
                        ElicitResultContent::Number(n)
                    } else if let Some(b) = v.as_bool() {
                        ElicitResultContent::Boolean(b)
                    } else {
                        return None;
                    };
                    Some((k.clone(), content))
                })
                .collect()
        })
    });

    Ok(ElicitResult {
        meta,
        action,
        content,
    })
}

/// Parse cancellation notification parameters.
fn parse_cancellation(params: &Value) -> Result<Cancellation, String> {
    let request_id = params
        .get("requestId")
        .ok_or_else(|| "Missing 'requestId' in cancellation".to_string())
        .and_then(|id| {
            if let Some(n) = id.as_i64() {
                Ok(Id::Number(n))
            } else if let Some(s) = id.as_str() {
                Ok(Id::String(s.to_string()))
            } else {
                Err("Invalid 'requestId' type".to_string())
            }
        })?;

    let reason = params
        .get("reason")
        .and_then(|r| r.as_str())
        .map(String::from);

    Ok(Cancellation { request_id, reason })
}

impl bindings::exports::wasmcp::mcp::request::Guest for Component {
    type Request = Request;

    fn register_capabilities(
        request: RequestBorrow<'_>,
        capabilities: ServerCapabilities,
    ) {
        // This is a no-op in the request component itself.
        // The actual capability aggregation happens in the initialize-handler
        // which collects capabilities from all middleware components.
        let _ = (request, capabilities);
    }
}

bindings::export!(Component with_types_in bindings);