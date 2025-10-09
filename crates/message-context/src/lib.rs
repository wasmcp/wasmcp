//! Context component for the Model Context Protocol (MCP)
//!
//! This component provides JSON-RPC 2.0 message parsing and context management for MCP handlers.
//! It is responsible for:
//! - Parsing JSON-RPC 2.0 messages (requests, McpNotifications, results, errors) from byte arrays
//! - Validating message structure according to the JSON-RPC 2.0 specification
//! - Managing request-scoped context for middleware communication
//! - Providing strongly-typed access to the parsed message data
//!
//! # Architecture
//!
//! The context component acts as the entry point for all JSON-RPC messages,
//! converting raw bytes into a strongly-typed `jsonrpc-object` variant that handlers
//! can process. It maintains a request-scoped context that middleware
//! components can use to share state during message processing.
//!
//! # Example
//!
//! ```
//! // Transport component reads bytes from network/stdio
//! let bytes = read_from_transport();
//!
//! // Parse into a Context object
//! let context = Context::from_bytes(bytes)?;
//!
//! // Access the JSON-RPC message
//! let message = context.data();
//! match message {
//!     McpMessage::Request(req) => handle_request(req),
//!     McpMessage::McpNotification(notif) => handle_McpNotification(notif),
//!     McpMessage::Result(res) => handle_result(res),
//!     McpMessage::Error(err) => handle_error(err),
//! }
//! ```

mod bindings {
    wit_bindgen::generate!({
        world: "message-context",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp::message_context::{Guest, GuestMessageContext, MessageContext, ParseError};
use bindings::wasmcp::mcp::protocol::{
    ArgParams, Cancellation, ClientCapabilities, CompleteParams, CompletionArgument,
    CompletionContext, CompletionPromptReference, CompletionRef, ElicitResult,
    ElicitResultAction, ElicitResultContent, McpError, ErrorCode, Id, Implementation,
    InitializeParams, McpMessage, ListChangedCapabilityOption, McpNotification, NotificationMethod,
    ProgressToken, ProtocolVersion, McpRequest, RequestMethod, ResponseResult, McpResult,
    ServerCapability,
};

use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;

struct Component;

/// Internal representation of an MCP context.
///
/// This struct holds the parsed JSON-RPC message and provides
/// thread-safe access to request-scoped context.
struct Context {
    /// The parsed JSON-RPC object
    message: McpMessage,
    /// Request-scoped context for middleware communication
    context: RwLock<HashMap<String, String>>,
}

impl GuestMessageContext for Context {
    /// Parse a JSON-RPC 2.0 message from a byte array.
    ///
    /// This method validates the JSON structure and ensures it conforms
    /// to the JSON-RPC 2.0 specification before creating a Context object.
    fn from_bytes(bytes: Vec<u8>) -> Result<MessageContext, ParseError> {
        // Parse as generic JSON first
        let parsed: Value = serde_json::from_slice(&bytes)
            .map_err(|e| ParseError::InvalidJson(format!("JSON parse error: {}", e)))?;

        // Validate it's a JSON object
        if !parsed.is_object() {
            return Err(ParseError::InvalidJson("Not a valid JSON object".to_string()));
        }

        // Check for JSON-RPC 2.0 version
        let jsonrpc_version = parsed.get("jsonrpc").and_then(|v| v.as_str());

        // Parse based on what fields are present
        let jsonrpc = if parsed.get("method").is_some() {
            // It's either a request or McpNotification
            if jsonrpc_version != Some("2.0") {
                return Err(ParseError::InvalidJsonrpc("Invalid or missing jsonrpc version for request/McpNotification".to_string()));
            }

            if let Some(_id_value) = parsed.get("id") {
                // It's a request (has id)
                parse_request(&parsed)
            } else {
                // It's a McpNotification (no id)
                parse_notification(&parsed)
            }
        } else if parsed.get("result").is_some() {
            // It's a result response
            if jsonrpc_version != Some("2.0") {
                return Err(ParseError::InvalidJsonrpc("Invalid or missing jsonrpc version for result".to_string()));
            }
            parse_result(&parsed)
        } else if parsed.get("error").is_some() {
            // It's an error response
            if jsonrpc_version != Some("2.0") {
                return Err(ParseError::InvalidJsonrpc("Invalid or missing jsonrpc version for error".to_string()));
            }
            parse_error(&parsed)
        } else {
            return Err(ParseError::InvalidJsonrpc("Unrecognized JSON-RPC message format".to_string()));
        };

        let message = jsonrpc.map_err(|e| ParseError::InvalidJsonrpc(e))?;

        // Create context with parsed JSON-RPC object
        Ok(MessageContext::new(
            Context {
                message,
                context: RwLock::new(HashMap::new()),
            },
        ))
    }

    /// Get the parsed JSON-RPC object.
    fn message(&self) -> McpMessage {
        self.message.clone()
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

    /// Register server capabilities in the context store.
    ///
    /// This provides a type-safe way for middleware components to register
    /// their capabilities. Each non-None capability is serialized to JSON
    /// and stored with the key "wasmcp:capability:{name}".
    fn register_capability(&self, capability: ServerCapability) {
        const PREFIX: &str = "wasmcp:capability:";

        if let Ok(mut ctx) = self.context.write() {
            // Register tools capability
            if let ServerCapability::Tools(Some(tools)) = &capability {
                let mut json = serde_json::Map::new();
                if let Some(list_changed) = tools.list_changed {
                    json.insert("listChanged".to_string(), serde_json::Value::Bool(list_changed));
                }
                ctx.insert(format!("{}tools", PREFIX), serde_json::Value::Object(json).to_string());
            }

            // Register prompts capability
            if let ServerCapability::Prompts(Some(prompts)) = &capability {
                let mut json = serde_json::Map::new();
                if let Some(list_changed) = prompts.list_changed {
                    json.insert("listChanged".to_string(), serde_json::Value::Bool(list_changed));
                }
                ctx.insert(format!("{}prompts", PREFIX), serde_json::Value::Object(json).to_string());
            }

            // Register resources capability
            if let ServerCapability::Resources(Some(resources)) = &capability {
                let mut json = serde_json::Map::new();
                if let Some(list_changed) = resources.list_changed {
                    json.insert("listChanged".to_string(), serde_json::Value::Bool(list_changed));
                }
                if let Some(subscribe) = resources.subscribe {
                    json.insert("subscribe".to_string(), serde_json::Value::Bool(subscribe));
                }
                ctx.insert(format!("{}resources", PREFIX), serde_json::Value::Object(json).to_string());
            }

            // Register completions capability
            if let ServerCapability::Completions(Some(completions)) = &capability {
                ctx.insert(format!("{}completions", PREFIX), completions.clone());
            }

            // Register logging capability
            if let ServerCapability::Logging(Some(logging)) = &capability {
                ctx.insert(format!("{}logging", PREFIX), logging.clone());
            }

            // Register experimental capabilities
            if let ServerCapability::Experimental(Some(experimental)) = &capability {
                let json = serde_json::to_string(&experimental).unwrap_or_else(|_| "[]".to_string());
                ctx.insert(format!("{}experimental", PREFIX), json);
            }
        }
    }
}

// === Helper Parsing Functions ===

/// Parse a JSON-RPC request
fn parse_request(parsed: &Value) -> Result<McpMessage, String> {
    // Parse ID
    let id = parsed
        .get("id")
        .ok_or("Request missing id field")?;

    let id = parse_id(id)?;

    // Parse method and params
    let method_str = parsed
        .get("method")
        .and_then(|m| m.as_str())
        .ok_or("Request missing method field")?;

    let params = parsed.get("params");

    // Parse request method based on the method string
    let method = match method_str {
        "initialize" => {
            if let Some(params) = params {
                RequestMethod::Initialize(parse_initialize_params(params)?)
            } else {
                return Err("initialize request missing params".to_string());
            }
        }
        "tools/list" => {
            let cursor = params
                .and_then(|p| p.get("cursor"))
                .and_then(|c| c.as_str())
                .map(|s| s.to_string());
            RequestMethod::ToolsList(cursor)
        }
        "tools/call" => {
            if let Some(params) = params {
                RequestMethod::ToolsCall(parse_tools_call_params(params)?)
            } else {
                return Err("tools/call request missing params".to_string());
            }
        }
        "resources/list" => {
            let cursor = params
                .and_then(|p| p.get("cursor"))
                .and_then(|c| c.as_str())
                .map(|s| s.to_string());
            RequestMethod::ResourcesList(cursor)
        }
        "resources/read" => {
            let uri = params
                .and_then(|p| p.get("uri"))
                .and_then(|u| u.as_str())
                .ok_or("resources/read missing uri param")?
                .to_string();
            RequestMethod::ResourcesRead(uri)
        }
        "resources/templates/list" => {
            let cursor = params
                .and_then(|p| p.get("cursor"))
                .and_then(|c| c.as_str())
                .map(|s| s.to_string());
            RequestMethod::ResourcesTemplatesList(cursor)
        }
        "prompts/list" => {
            let cursor = params
                .and_then(|p| p.get("cursor"))
                .and_then(|c| c.as_str())
                .map(|s| s.to_string());
            RequestMethod::PromptsList(cursor)
        }
        "prompts/get" => {
            if let Some(params) = params {
                RequestMethod::PromptsGet(parse_prompts_get_params(params)?)
            } else {
                return Err("prompts/get request missing params".to_string());
            }
        }
        "completion/complete" => {
            if let Some(params) = params {
                RequestMethod::CompletionComplete(parse_complete_params(params)?)
            } else {
                return Err("completion/complete request missing params".to_string());
            }
        }
        "ping" => RequestMethod::Ping,
        _ => return Err(format!("Unknown request method: {}", method_str)),
    };

    // Parse progress token if present
    let progress_token = parsed
        .get("_meta")
        .and_then(|meta| meta.get("progressToken"))
        .and_then(|token| parse_progress_token(token).ok());

    Ok(McpMessage::Request(McpRequest {
        id,
        method,
        progress_token,
    }))
}

/// Parse a JSON-RPC McpNotification
fn parse_notification(parsed: &Value) -> Result<McpMessage, String> {
    let method_str = parsed
        .get("method")
        .and_then(|m| m.as_str())
        .ok_or("McpNotification missing method field")?;

    let params = parsed.get("params");

    let method = match method_str {
        "McpNotifications/cancelled" => {
            if let Some(params) = params {
                NotificationMethod::Cancellation(parse_cancellation(params)?)
            } else {
                return Err("cancellation McpNotification missing params".to_string());
            }
        }
        "McpNotifications/progress" => {
            let token = params
                .and_then(|p| p.get("progressToken"))
                .ok_or("progress McpNotification missing progressToken")?;
            NotificationMethod::Progress(parse_progress_token(token)?)
        }
        "McpNotifications/initialized" => NotificationMethod::Initialized,
        "roots/list_changed" => NotificationMethod::RootsListChanged,
        _ => return Err(format!("Unknown McpNotification method: {}", method_str)),
    };

    Ok(McpMessage::Notification(McpNotification { method }))
}

/// Parse a JSON-RPC result
fn parse_result(parsed: &Value) -> Result<McpMessage, String> {
    let id = parsed
        .get("id")
        .ok_or("Result missing id field")?;

    let id = parse_id(id)?;

    let result_value = parsed
        .get("result")
        .ok_or("Result missing result field")?;

    // For now, we only support elicit-result as a response type
    // In the future, this could be extended based on context or other indicators
    let result = if result_value.get("action").is_some() {
        // It looks like an elicit result
        ResponseResult::ElicitResult(parse_elicit_result(result_value)?)
    } else {
        return Err("Unknown result type".to_string());
    };

    Ok(McpMessage::Result(McpResult { id, result }))
}

/// Parse a JSON-RPC error
fn parse_error(parsed: &Value) -> Result<McpMessage, String> {
    let id = parsed.get("id").and_then(|id| parse_id(id).ok());

    let error_obj = parsed
        .get("error")
        .ok_or("Error missing error field")?;

    let code = error_obj
        .get("code")
        .and_then(|c| c.as_i64())
        .ok_or("Error missing code field")?;

    let error_code = match code {
        -32700 => ErrorCode::ParseError,
        -32600 => ErrorCode::InvalidRequest,
        -32601 => ErrorCode::MethodNotFound,
        -32602 => ErrorCode::InvalidParams,
        -32603 => ErrorCode::InternalError,
        _ => ErrorCode::InternalError, // Default for unknown codes
    };

    let message = error_obj
        .get("message")
        .and_then(|m| m.as_str())
        .ok_or("Error missing message field")?
        .to_string();

    let data = error_obj
        .get("data")
        .map(|d| serde_json::to_string(d).unwrap_or_else(|_| d.to_string()));

    Ok(McpMessage::Error(McpError {
        id,
        code: error_code,
        message,
        data,
    }))
}

/// Parse an ID value (string or number)
fn parse_id(value: &Value) -> Result<Id, String> {
    if let Some(num) = value.as_i64() {
        Ok(Id::Number(num))
    } else if let Some(s) = value.as_str() {
        Ok(Id::String(s.to_string()))
    } else {
        Err("Invalid id type (must be string or number)".to_string())
    }
}

/// Parse a progress token (string or number)
fn parse_progress_token(value: &Value) -> Result<ProgressToken, String> {
    if let Some(s) = value.as_str() {
        Ok(ProgressToken::String(s.to_string()))
    } else if let Some(n) = value.as_i64() {
        Ok(ProgressToken::Integer(n))
    } else {
        Err("Invalid progress token type".to_string())
    }
}

/// Parse initialize request parameters
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
            // Parse elicitation capability (if present, it's an object)
            let elicitation = caps
                .get("elicitation")
                .map(|e| serde_json::to_string(e).unwrap_or_else(|_| "{}".to_string()));

            // Parse roots capability (with listChanged option)
            let roots = caps.get("roots").and_then(|r| {
                Some(ListChangedCapabilityOption {
                    list_changed: r.get("listChanged").and_then(|lc| lc.as_bool()),
                })
            });

            // Parse sampling capability (if present, it's an object)
            let sampling = caps
                .get("sampling")
                .map(|s| serde_json::to_string(s).unwrap_or_else(|_| "{}".to_string()));

            // Parse experimental capabilities
            let experimental = caps.get("experimental").and_then(|exp| {
                if let Some(obj) = exp.as_object() {
                    Some(
                        obj.iter()
                            .map(|(k, v)| {
                                (
                                    k.clone(),
                                    serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string()),
                                )
                            })
                            .collect(),
                    )
                } else {
                    None
                }
            });

            ClientCapabilities {
                elicitation,
                roots,
                sampling,
                experimental,
            }
        })
        .unwrap_or(ClientCapabilities {
            elicitation: None,
            roots: None,
            sampling: None,
            experimental: None,
        });

    Ok(InitializeParams {
        capabilities,
        client_info,
        protocol_version,
    })
}

/// Parse tools/call request parameters
fn parse_tools_call_params(params: &Value) -> Result<ArgParams, String> {
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| "Missing 'name' in tools/call params".to_string())?
        .to_string();

    let arguments = params
        .get("arguments")
        .map(|a| serde_json::to_string(a).unwrap_or_else(|_| a.to_string()));

    Ok(ArgParams { name, arguments })
}

/// Parse prompts/get request parameters
fn parse_prompts_get_params(params: &Value) -> Result<ArgParams, String> {
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| "Missing 'name' in prompts/get params".to_string())?
        .to_string();

    let arguments = params
        .get("arguments")
        .map(|a| serde_json::to_string(a).unwrap_or_else(|_| a.to_string()));

    Ok(ArgParams { name, arguments })
}

/// Parse completion/complete request parameters
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

/// Parse elicit/result parameters
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

/// Parse cancellation McpNotification parameters
fn parse_cancellation(params: &Value) -> Result<Cancellation, String> {
    let request_id = params
        .get("requestId")
        .ok_or_else(|| "Missing 'requestId' in cancellation".to_string())
        .and_then(|id| parse_id(id))?;

    let reason = params
        .get("reason")
        .and_then(|r| r.as_str())
        .map(String::from);

    Ok(Cancellation {
        request_id,
        reason,
    })
}

impl Guest for Component {
    type MessageContext = Context;
}

bindings::export!(Component with_types_in bindings);