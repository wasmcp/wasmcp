//! HTTP transport component for the Model Context Protocol (MCP)
//!
//! This component provides a Server-Sent Events (SSE) HTTP transport implementation
//! that converts incoming HTTP requests into MCP protocol messages for processing
//! by handler chains.
//!
//! ## Architecture
//!
//! The transport implements the `wasmcp:mcp/transport` world by:
//! - Exporting `context` interface for request-scoped state management
//! - Exporting `output` interface for SSE-framed message writing
//! - Importing `message-handler` to delegate parsed messages to handler chains
//!
//! State is managed per-request using thread-local storage, which is safe in the
//! single-threaded WASI execution model.
//!
//! ## Request Flow
//!
//! 1. HTTP request arrives at `/mcp` endpoint
//! 2. Body is read and parsed as JSON-RPC 2.0 into `mcp-message`
//! 3. Per-request state is initialized (output stream, context KV store)
//! 4. Message is forwarded to the composed handler chain via `message-handler::handle()`
//! 5. Handlers access state through imported `context` and `output` functions
//! 6. These function calls route back to this transport's exported implementations
//! 7. Response is written with SSE framing and the stream is cleaned up

use std::cell::RefCell;
use std::collections::HashMap;

mod bindings {
    wit_bindgen::generate!({
        world: "http-transport",
        generate_all,
    });
}

use bindings::exports::wasi::http::incoming_handler::{Guest, IncomingRequest, ResponseOutparam};
use bindings::exports::wasmcp::mcp::context::Guest as ContextGuest;
use bindings::exports::wasmcp::mcp::output::{Guest as OutputGuest, IoError};
use bindings::wasi::http::types::{
    Headers, IncomingBody, Method, OutgoingBody, OutgoingResponse,
    ResponseOutparam as WasiResponseOutparam,
};
use bindings::wasi::io::streams::{OutputStream, StreamError};
use bindings::wasmcp::mcp::message_handler::handle;
use bindings::wasmcp::mcp::protocol::{self as mcp, McpMessage, ServerCapability};

use serde_json::Value;

// Per-request state stored in thread-local storage.
// This is safe because WASI components are single-threaded and each request
// is processed sequentially.
thread_local! {
    /// The output stream for the current request's SSE response
    static OUTPUT_STREAM: RefCell<Option<OutputStream>> = const { RefCell::new(None) };

    /// Key-value context storage accessible to handlers via context::get/set
    static CONTEXT_STORE: RefCell<HashMap<String, Vec<u8>>> = RefCell::new(HashMap::new());

    /// Registered server capabilities for this request
    static CAPABILITIES: RefCell<Vec<ServerCapability>> = const { RefCell::new(Vec::new()) };

    /// Track message lifecycle state for output interface
    static MESSAGE_STATE: RefCell<MessageState> = const { RefCell::new(MessageState::NotStarted) };

    /// Cached allowed origins loaded from wasi-config
    /// Loaded once and cached for component lifetime
    static ALLOWED_ORIGINS: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageState {
    NotStarted,
    InProgress,
    Finished,
}

struct Component;

impl Guest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        // Validate HTTP method is POST
        let method = request.method();
        if !matches!(method, Method::Post) {
            send_error_response(
                response_out,
                405,
                b"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32600,\"message\":\"Method Not Allowed: Only POST requests are supported\"},\"id\":null}",
            );
            return;
        }

        // Validate path is /mcp
        let path = request.path_with_query().unwrap_or_default();
        if !path.starts_with("/mcp") {
            send_error_response(
                response_out,
                404,
                b"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32600,\"message\":\"Not Found: MCP endpoint is /mcp\"},\"id\":null}",
            );
            return;
        }

        let headers = request.headers();

        // Validate Origin header to prevent DNS rebinding attacks
        // Spec: Servers MUST validate the Origin header on all incoming connections
        let origin_header = headers.get(&"origin".to_string());
        if !origin_header.is_empty() {
            let origin = std::str::from_utf8(&origin_header[0]).unwrap_or("");
            // Allow localhost origins only (security requirement for local servers)
            // This is configurable via wasi-config "allowed-origins"
            if !is_allowed_origin(origin) {
                send_error_response(
                    response_out,
                    403,
                    b"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32600,\"message\":\"Forbidden: Origin not allowed\"},\"id\":null}",
                );
                return;
            }
        }

        // Validate MCP-Protocol-Version header
        // Spec: Client MUST include MCP-Protocol-Version header on all subsequent requests
        let protocol_version_header = headers.get(&"mcp-protocol-version".to_string());
        let protocol_version = protocol_version_header
            .first()
            .and_then(|v| std::str::from_utf8(v).ok());

        // Validate protocol version if present
        if let Some(version) = protocol_version {
            if !is_supported_protocol_version(version) {
                send_error_response(
                    response_out,
                    400,
                    b"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32600,\"message\":\"Bad Request: Unsupported MCP protocol version\"},\"id\":null}",
                );
                return;
            }
        }
        // Note: Per spec, if no header present, assume 2025-03-26 for backwards compatibility

        // Validate Accept header for SSE support
        let accept_header = headers.get(&"accept".to_string());
        let accepts_sse = accept_header.iter().any(|value| {
            std::str::from_utf8(value)
                .map(|s| {
                    s.contains("text/event-stream")
                        || s.contains("application/json")
                        || s.contains("*/*")
                })
                .unwrap_or(false)
        });

        if !accepts_sse {
            send_error_response(
                response_out,
                406,
                b"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32600,\"message\":\"Not Acceptable: Client must accept text/event-stream or application/json\"},\"id\":null}",
            );
            return;
        }

        // Read request body
        let incoming_body = match request.consume() {
            Ok(body) => body,
            Err(_) => {
                send_error_response(
                    response_out,
                    400,
                    b"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32600,\"message\":\"Bad Request: Unable to consume request body\"},\"id\":null}",
                );
                return;
            }
        };

        let content_length = extract_content_length(&headers);
        let is_chunked = is_chunked_encoding(&headers);

        if content_length.is_none() && !is_chunked {
            send_error_response(
                response_out,
                411,
                b"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32600,\"message\":\"Length Required: Content-Length header or chunked encoding required\"},\"id\":null}",
            );
            return;
        }

        // Read body to bytes
        let bytes = match read_body_to_bytes(incoming_body, content_length, is_chunked) {
            Ok(bytes) => bytes,
            Err(_) => {
                send_error_response(
                    response_out,
                    400,
                    b"{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32700,\"message\":\"Parse error: Failed to read request body\"},\"id\":null}",
                );
                return;
            }
        };

        // Parse JSON to mcp-message
        let message = match parse_mcp_message(&bytes) {
            Ok(msg) => msg,
            Err(e) => {
                let error_msg = format!(
                    "{{\"jsonrpc\":\"2.0\",\"error\":{{\"code\":-32700,\"message\":\"Parse error: {}\"}},\"id\":null}}",
                    e
                );
                send_error_response(response_out, 400, error_msg.as_bytes());
                return;
            }
        };

        // Determine if we need SSE response (requests do, notifications don't)
        let use_sse = matches!(message, McpMessage::Request(_));

        // Create response with appropriate headers
        let headers = Headers::new();
        if use_sse {
            // Set SSE headers only for 200 responses
            headers
                .set(
                    &"content-type".to_string(),
                    &[b"text/event-stream".to_vec()],
                )
                .expect("Failed to set content-type header");
            headers
                .set(&"cache-control".to_string(), &[b"no-cache".to_vec()])
                .expect("Failed to set cache-control header");
            headers
                .set(&"x-accel-buffering".to_string(), &[b"no".to_vec()])
                .expect("Failed to set x-accel-buffering header");
        }
        // For 202 responses (notifications), no headers needed per spec

        let response = OutgoingResponse::new(headers);
        let status = if use_sse { 200 } else { 202 };
        response
            .set_status_code(status)
            .expect("Failed to set status code");

        let response_body = response.body().expect("Failed to get response body");

        // Set the response before processing (required by WASI HTTP)
        WasiResponseOutparam::set(response_out, Ok(response));

        if use_sse {
            // Get output stream and store in thread-local
            let output_stream = response_body
                .write()
                .expect("Failed to get output stream from response body");

            // Initialize per-request state
            OUTPUT_STREAM.with(|s| {
                *s.borrow_mut() = Some(output_stream);
            });
            CONTEXT_STORE.with(|s| {
                s.borrow_mut().clear();
            });
            CAPABILITIES.with(|c| {
                c.borrow_mut().clear();
            });
            MESSAGE_STATE.with(|s| {
                *s.borrow_mut() = MessageState::NotStarted;
            });

            // Forward to handler chain with panic recovery
            let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                handle(&message);
            }));

            // If handler panicked, send error response if possible
            if panic_result.is_err() {
                send_panic_error_response();
            }

            // Always clean up state, even after panic
            OUTPUT_STREAM.with(|s| {
                *s.borrow_mut() = None;
            });
            MESSAGE_STATE.with(|s| {
                *s.borrow_mut() = MessageState::NotStarted;
            });
        } else {
            // For notifications, forward without SSE (panics are fatal for notifications)
            handle(&message);
        }

        // Finish the response body
        OutgoingBody::finish(response_body, None).expect("Failed to finish response body");
    }
}

impl ContextGuest for Component {
    fn get(key: String) -> Option<Vec<u8>> {
        CONTEXT_STORE.with(|store| store.borrow().get(&key).cloned())
    }

    fn set(key: String, value: Vec<u8>) {
        CONTEXT_STORE.with(|store| {
            store.borrow_mut().insert(key, value);
        });
    }

    fn register_capability(capability: ServerCapability) {
        CAPABILITIES.with(|caps| {
            caps.borrow_mut().push(capability);
        });
    }
}

impl OutputGuest for Component {
    fn start_message() -> Result<(), IoError> {
        // Check state and transition to InProgress
        MESSAGE_STATE.with(|state| {
            let current = *state.borrow();
            match current {
                MessageState::NotStarted => {
                    *state.borrow_mut() = MessageState::InProgress;
                    Ok(())
                }
                MessageState::InProgress => Err(IoError::MessageInProgress),
                MessageState::Finished => Err(IoError::MessageFinished),
            }
        })?;

        // Write SSE "data: " prefix
        OUTPUT_STREAM.with(|stream_cell| {
            let stream_opt = stream_cell.borrow();
            let stream = stream_opt
                .as_ref()
                .ok_or_else(|| IoError::Unexpected("No output stream available".to_string()))?;

            write_with_backpressure(stream, b"data: ")
                .map_err(|e| IoError::Io(format!("Stream error: {:?}", e)))
        })
    }

    fn write_message_contents(contents: Vec<u8>) -> Result<(), IoError> {
        // Check state
        MESSAGE_STATE.with(|state| {
            let current = *state.borrow();
            match current {
                MessageState::NotStarted => Err(IoError::MessageNotStarted),
                MessageState::InProgress => Ok(()),
                MessageState::Finished => Err(IoError::MessageFinished),
            }
        })?;

        // Write raw bytes without any framing
        OUTPUT_STREAM.with(|stream_cell| {
            let stream_opt = stream_cell.borrow();
            let stream = stream_opt
                .as_ref()
                .ok_or_else(|| IoError::Unexpected("No output stream available".to_string()))?;

            write_with_backpressure(stream, &contents)
                .map_err(|e| IoError::Io(format!("Stream error: {:?}", e)))
        })
    }

    fn finish_message() -> Result<(), IoError> {
        // Check state and transition to Finished
        MESSAGE_STATE.with(|state| {
            let current = *state.borrow();
            match current {
                MessageState::NotStarted => Err(IoError::MessageNotStarted),
                MessageState::InProgress => {
                    *state.borrow_mut() = MessageState::Finished;
                    Ok(())
                }
                MessageState::Finished => Err(IoError::MessageFinished),
            }
        })?;

        // Write SSE terminator "\n\n" and flush
        OUTPUT_STREAM.with(|stream_cell| {
            let stream_opt = stream_cell.borrow();
            let stream = stream_opt
                .as_ref()
                .ok_or_else(|| IoError::Unexpected("No output stream available".to_string()))?;

            write_with_backpressure(stream, b"\n\n")
                .and_then(|_| stream.flush())
                .map_err(|e| IoError::Io(format!("Stream error: {:?}", e)))
        })
    }
}

// === MCP Message Parsing ===

/// Parse JSON-RPC 2.0 bytes into an MCP message variant.
fn parse_mcp_message(bytes: &[u8]) -> Result<McpMessage, String> {
    let parsed: Value =
        serde_json::from_slice(bytes).map_err(|e| format!("JSON parse error: {e}"))?;

    if !parsed.is_object() {
        return Err("Not a valid JSON object".to_string());
    }

    let jsonrpc_version = parsed.get("jsonrpc").and_then(|v| v.as_str());

    // Parse based on message structure
    if parsed.get("method").is_some() {
        // It's either a request or notification
        if jsonrpc_version != Some("2.0") {
            return Err("Invalid or missing jsonrpc version for request/notification".to_string());
        }

        if parsed.get("id").is_some() {
            parse_request(&parsed)
        } else {
            parse_notification(&parsed)
        }
    } else if parsed.get("result").is_some() {
        if jsonrpc_version != Some("2.0") {
            return Err("Invalid or missing jsonrpc version for result".to_string());
        }
        parse_result(&parsed)
    } else if parsed.get("error").is_some() {
        if jsonrpc_version != Some("2.0") {
            return Err("Invalid or missing jsonrpc version for error".to_string());
        }
        parse_error(&parsed)
    } else {
        Err("Unrecognized JSON-RPC message format".to_string())
    }
}

/// Parse a JSON-RPC request.
fn parse_request(parsed: &Value) -> Result<McpMessage, String> {
    let id = parsed.get("id").ok_or("Request missing id field")?;
    let id = parse_id(id)?;

    let method_str = parsed
        .get("method")
        .and_then(|m| m.as_str())
        .ok_or("Request missing method field")?;

    let params = parsed.get("params");

    let method = match method_str {
        "initialize" => {
            let params = params.ok_or("initialize request missing params")?;
            mcp::RequestMethod::Initialize(parse_initialize_params(params)?)
        }
        "tools/list" => {
            let cursor = params
                .and_then(|p| p.get("cursor"))
                .and_then(|c| c.as_str())
                .map(|s| s.to_string());
            mcp::RequestMethod::ToolsList(cursor)
        }
        "tools/call" => {
            let params = params.ok_or("tools/call request missing params")?;
            mcp::RequestMethod::ToolsCall(parse_arg_params(params)?)
        }
        "resources/list" => {
            let cursor = params
                .and_then(|p| p.get("cursor"))
                .and_then(|c| c.as_str())
                .map(|s| s.to_string());
            mcp::RequestMethod::ResourcesList(cursor)
        }
        "resources/read" => {
            let uri = params
                .and_then(|p| p.get("uri"))
                .and_then(|u| u.as_str())
                .ok_or("resources/read missing uri param")?
                .to_string();
            mcp::RequestMethod::ResourcesRead(uri)
        }
        "resources/templates/list" => {
            let cursor = params
                .and_then(|p| p.get("cursor"))
                .and_then(|c| c.as_str())
                .map(|s| s.to_string());
            mcp::RequestMethod::ResourcesTemplatesList(cursor)
        }
        "prompts/list" => {
            let cursor = params
                .and_then(|p| p.get("cursor"))
                .and_then(|c| c.as_str())
                .map(|s| s.to_string());
            mcp::RequestMethod::PromptsList(cursor)
        }
        "prompts/get" => {
            let params = params.ok_or("prompts/get request missing params")?;
            mcp::RequestMethod::PromptsGet(parse_arg_params(params)?)
        }
        "completion/complete" => {
            let params = params.ok_or("completion/complete request missing params")?;
            mcp::RequestMethod::CompletionComplete(parse_complete_params(params)?)
        }
        "ping" => mcp::RequestMethod::Ping,
        _ => return Err(format!("Unknown request method: {method_str}")),
    };

    let progress_token = parsed
        .get("_meta")
        .and_then(|meta| meta.get("progressToken"))
        .and_then(|token| parse_progress_token(token).ok());

    Ok(McpMessage::Request(mcp::McpRequest {
        id,
        method,
        progress_token,
    }))
}

/// Parse a JSON-RPC notification.
fn parse_notification(parsed: &Value) -> Result<McpMessage, String> {
    let method_str = parsed
        .get("method")
        .and_then(|m| m.as_str())
        .ok_or("Notification missing method field")?;

    let params = parsed.get("params");

    let method = match method_str {
        "notifications/cancelled" => {
            let params = params.ok_or("cancellation notification missing params")?;
            mcp::NotificationMethod::Cancellation(parse_cancellation(params)?)
        }
        "notifications/progress" => {
            let token = params
                .and_then(|p| p.get("progressToken"))
                .ok_or("progress notification missing progressToken")?;
            mcp::NotificationMethod::Progress(parse_progress_token(token)?)
        }
        "notifications/initialized" => mcp::NotificationMethod::Initialized,
        "roots/list_changed" => mcp::NotificationMethod::RootsListChanged,
        _ => return Err(format!("Unknown notification method: {method_str}")),
    };

    Ok(McpMessage::Notification(mcp::McpNotification { method }))
}

/// Parse a JSON-RPC result.
fn parse_result(parsed: &Value) -> Result<McpMessage, String> {
    let id = parsed.get("id").ok_or("Result missing id field")?;
    let id = parse_id(id)?;

    let result_value = parsed.get("result").ok_or("Result missing result field")?;

    // Detect elicit-result by presence of action field
    let result = if result_value.get("action").is_some() {
        mcp::ResponseResult::ElicitResult(parse_elicit_result(result_value)?)
    } else {
        return Err("Unknown result type".to_string());
    };

    Ok(McpMessage::Result(mcp::McpResult { id, result }))
}

/// Parse a JSON-RPC error.
fn parse_error(parsed: &Value) -> Result<McpMessage, String> {
    let id = parsed.get("id").and_then(|id| parse_id(id).ok());

    let error_obj = parsed.get("error").ok_or("Error missing error field")?;

    let code = error_obj
        .get("code")
        .and_then(|c| c.as_i64())
        .ok_or("Error missing code field")?;

    let error_code = match code {
        -32700 => mcp::ErrorCode::ParseError,
        -32600 => mcp::ErrorCode::InvalidRequest,
        -32601 => mcp::ErrorCode::MethodNotFound,
        -32602 => mcp::ErrorCode::InvalidParams,
        -32603 => mcp::ErrorCode::InternalError,
        _ => mcp::ErrorCode::InternalError,
    };

    let message = error_obj
        .get("message")
        .and_then(|m| m.as_str())
        .ok_or("Error missing message field")?
        .to_string();

    let data = error_obj
        .get("data")
        .map(|d| serde_json::to_string(d).unwrap_or_else(|_| d.to_string()));

    Ok(McpMessage::Error(mcp::McpError {
        id,
        code: error_code,
        message,
        data,
    }))
}

// === Parsing Helper Functions ===

fn parse_id(value: &Value) -> Result<mcp::Id, String> {
    if let Some(num) = value.as_i64() {
        Ok(mcp::Id::Number(num))
    } else if let Some(s) = value.as_str() {
        Ok(mcp::Id::String(s.to_string()))
    } else {
        Err("Invalid id type (must be string or number)".to_string())
    }
}

fn parse_progress_token(value: &Value) -> Result<mcp::ProgressToken, String> {
    if let Some(s) = value.as_str() {
        Ok(mcp::ProgressToken::String(s.to_string()))
    } else if let Some(n) = value.as_i64() {
        Ok(mcp::ProgressToken::Integer(n))
    } else {
        Err("Invalid progress token type".to_string())
    }
}

fn parse_initialize_params(params: &Value) -> Result<mcp::InitializeParams, String> {
    let protocol_version = params
        .get("protocolVersion")
        .and_then(|v| v.as_str())
        .map(|s| match s {
            "2025-06-18" => mcp::ProtocolVersion::V20250618,
            "2025-03-26" => mcp::ProtocolVersion::V20250326,
            "2024-11-05" => mcp::ProtocolVersion::V20241105,
            _ => mcp::ProtocolVersion::V20250618,
        })
        .unwrap_or(mcp::ProtocolVersion::V20250618);

    let client_info = params
        .get("clientInfo")
        .and_then(|ci| {
            let name = ci.get("name")?.as_str()?.to_string();
            let version = ci.get("version")?.as_str()?.to_string();
            let title = ci.get("title").and_then(|t| t.as_str()).map(String::from);
            Some(mcp::Implementation {
                name,
                version,
                title,
            })
        })
        .ok_or_else(|| "Missing or invalid clientInfo".to_string())?;

    let capabilities = params
        .get("capabilities")
        .map(|caps| {
            let elicitation = caps
                .get("elicitation")
                .map(|e| serde_json::to_string(e).unwrap_or_else(|_| "{}".to_string()));

            let roots = caps.get("roots").map(|r| mcp::ListChangedCapabilityOption {
                list_changed: r.get("listChanged").and_then(|lc| lc.as_bool()),
            });

            let sampling = caps
                .get("sampling")
                .map(|s| serde_json::to_string(s).unwrap_or_else(|_| "{}".to_string()));

            let experimental = caps.get("experimental").and_then(|exp| {
                exp.as_object().map(|obj| {
                    obj.iter()
                        .map(|(k, v)| {
                            (
                                k.clone(),
                                serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string()),
                            )
                        })
                        .collect()
                })
            });

            mcp::ClientCapabilities {
                elicitation,
                roots,
                sampling,
                experimental,
            }
        })
        .unwrap_or(mcp::ClientCapabilities {
            elicitation: None,
            roots: None,
            sampling: None,
            experimental: None,
        });

    Ok(mcp::InitializeParams {
        capabilities,
        client_info,
        protocol_version,
    })
}

fn parse_arg_params(params: &Value) -> Result<mcp::ArgParams, String> {
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| "Missing 'name' in params".to_string())?
        .to_string();

    let arguments = params
        .get("arguments")
        .map(|a| serde_json::to_string(a).unwrap_or_else(|_| a.to_string()));

    Ok(mcp::ArgParams { name, arguments })
}

fn parse_complete_params(params: &Value) -> Result<mcp::CompleteParams, String> {
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
            Ok(mcp::CompletionArgument { name, value })
        })?;

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
                Ok(mcp::CompletionRef::Prompt(mcp::CompletionPromptReference {
                    name,
                    title,
                }))
            } else if let Some(template) = r.get("resourceTemplate") {
                let uri = template.as_str().unwrap_or("").to_string();
                Ok(mcp::CompletionRef::ResourceTemplate(uri))
            } else {
                Err("Invalid 'ref' in completion params".to_string())
            }
        })?;

    let context = params.get("context").map(|ctx| {
        let arguments = ctx
            .get("arguments")
            .and_then(|args| args.as_str())
            .map(String::from);
        mcp::CompletionContext { arguments }
    });

    Ok(mcp::CompleteParams {
        argument,
        ref_,
        context,
    })
}

fn parse_elicit_result(params: &Value) -> Result<mcp::ElicitResult, String> {
    let meta = params.get("_meta").and_then(|m| {
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
            "accept" => mcp::ElicitResultAction::Accept,
            "decline" => mcp::ElicitResultAction::Decline,
            "cancel" => mcp::ElicitResultAction::Cancel,
            _ => mcp::ElicitResultAction::Cancel,
        })
        .unwrap_or(mcp::ElicitResultAction::Cancel);

    let content = params.get("content").and_then(|c| {
        c.as_object().map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| {
                    let content = if let Some(s) = v.as_str() {
                        mcp::ElicitResultContent::String(s.to_string())
                    } else if let Some(n) = v.as_f64() {
                        mcp::ElicitResultContent::Number(n)
                    } else if let Some(b) = v.as_bool() {
                        mcp::ElicitResultContent::Boolean(b)
                    } else {
                        return None;
                    };
                    Some((k.clone(), content))
                })
                .collect()
        })
    });

    Ok(mcp::ElicitResult {
        meta,
        action,
        content,
    })
}

fn parse_cancellation(params: &Value) -> Result<mcp::Cancellation, String> {
    let request_id = params
        .get("requestId")
        .ok_or_else(|| "Missing 'requestId' in cancellation".to_string())
        .and_then(parse_id)?;

    let reason = params
        .get("reason")
        .and_then(|r| r.as_str())
        .map(String::from);

    Ok(mcp::Cancellation { request_id, reason })
}

// === Stream Writing ===

/// Write bytes to stream with proper backpressure handling.
///
/// Respects the check-write contract by:
/// 1. Checking available space before writing
/// 2. Writing in chunks if necessary
/// 3. Handling zero-availability gracefully
fn write_with_backpressure(stream: &OutputStream, bytes: &[u8]) -> Result<(), StreamError> {
    let mut offset = 0;

    while offset < bytes.len() {
        let available = stream.check_write()?;

        if available == 0 {
            // Stream not ready - shouldn't happen in practice but handle gracefully
            continue;
        }

        let chunk_size = std::cmp::min(available as usize, bytes.len() - offset);
        stream.write(&bytes[offset..offset + chunk_size])?;
        offset += chunk_size;
    }

    Ok(())
}

// === HTTP Utilities ===

/// Extract Content-Length header value.
fn extract_content_length(headers: &Headers) -> Option<usize> {
    headers
        .get(&"content-length".to_string())
        .first()
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .and_then(|s| s.parse::<usize>().ok())
}

/// Check if Transfer-Encoding is chunked.
fn is_chunked_encoding(headers: &Headers) -> bool {
    headers
        .get(&"transfer-encoding".to_string())
        .first()
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(|s| s.to_lowercase().contains("chunked"))
        .unwrap_or(false)
}

/// Read an incoming body stream to bytes.
///
/// Handles both Content-Length and chunked Transfer-Encoding.
fn read_body_to_bytes(
    body: IncomingBody,
    content_length: Option<usize>,
    is_chunked: bool,
) -> Result<Vec<u8>, ()> {
    let stream = body.stream().map_err(|_| ())?;

    let mut bytes = if let Some(len) = content_length {
        Vec::with_capacity(len)
    } else {
        Vec::new()
    };

    if let Some(expected_length) = content_length {
        // Content-Length specified: read exactly that many bytes
        while bytes.len() < expected_length {
            let remaining = expected_length - bytes.len();
            let chunk_size = remaining.min(65536);

            match stream.blocking_read(chunk_size as u64) {
                Ok(chunk) => {
                    if chunk.is_empty() {
                        return Err(());
                    }
                    bytes.extend_from_slice(&chunk);
                }
                Err(_) => return Err(()),
            }
        }
    } else if is_chunked {
        // Transfer-Encoding: chunked - read until stream ends
        while let Ok(chunk) = stream.blocking_read(65536) {
            if chunk.is_empty() {
                break;
            }
            bytes.extend_from_slice(&chunk);
        }
    } else {
        return Err(());
    }

    // Consume the body to get trailers (ignored for MCP)
    let _ = IncomingBody::finish(body);

    Ok(bytes)
}

/// Send an error response with the given status code and message.
fn send_error_response(response_out: ResponseOutparam, status: u16, message: &[u8]) {
    let headers = Headers::new();
    headers
        .set(&"content-type".to_string(), &[b"application/json".to_vec()])
        .expect("Failed to set content-type header");

    let response = OutgoingResponse::new(headers);
    response
        .set_status_code(status)
        .expect("Failed to set status code");

    let response_body = response.body().expect("Failed to get response body");

    WasiResponseOutparam::set(response_out, Ok(response));

    {
        let output_stream = response_body
            .write()
            .expect("Failed to get output stream from response body");

        output_stream
            .blocking_write_and_flush(message)
            .expect("Failed to write error message");
    }

    OutgoingBody::finish(response_body, None).expect("Failed to finish error response body");
}

/// Validate Origin header to prevent DNS rebinding attacks.
///
/// Per MCP spec: Servers MUST validate the Origin header on all incoming connections.
///
/// Configuration:
/// - Reads "allowed-origins" from wasi:config (comma-separated list)
/// - Falls back to localhost-only if not configured (secure default)
/// - Fails closed on config errors (rejects request)
///
/// Examples:
/// - "https://app.example.com,https://api.example.com"
/// - "http://localhost,https://myapp.com"
fn is_allowed_origin(origin: &str) -> bool {
    ALLOWED_ORIGINS.with(|cache| {
        let mut cache_ref = cache.borrow_mut();

        // Lazy load: fetch config on first request and cache
        if cache_ref.is_none() {
            *cache_ref = Some(load_allowed_origins());
        }

        let allowed_origins = cache_ref.as_ref().unwrap();

        // Check if origin matches any allowed origin
        allowed_origins.iter().any(|allowed| {
            // Exact prefix match for security
            origin.starts_with(allowed.as_str())
        })
    })
}

/// Load allowed origins from wasi-config.
///
/// Config key: "allowed-origins"
/// Format: Comma-separated list of origin prefixes
///
/// Returns secure defaults (localhost-only) if:
/// - Config key not found
/// - Config value is empty
/// - Config fetch fails
fn load_allowed_origins() -> Vec<String> {
    use bindings::wasi::config::runtime as config;

    match config::get("allowed-origins") {
        Ok(Some(config_value)) => {
            if config_value.is_empty() {
                // Empty config: use secure default
                return get_localhost_origins();
            }

            // Parse comma-separated origins
            let origins: Vec<String> = config_value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            if origins.is_empty() {
                // Invalid config (only whitespace): use secure default
                get_localhost_origins()
            } else {
                // Validate each origin has a scheme
                let valid_origins: Vec<String> = origins
                    .into_iter()
                    .filter(|origin| {
                        origin.starts_with("http://") || origin.starts_with("https://")
                    })
                    .collect();

                if valid_origins.is_empty() {
                    // No valid origins: use secure default
                    get_localhost_origins()
                } else {
                    valid_origins
                }
            }
        }
        Ok(None) => {
            // Config key not found: use secure default (localhost-only)
            get_localhost_origins()
        }
        Err(_) => {
            // Config fetch failed: fail closed (reject all non-localhost)
            // This is the most secure option when config is unavailable
            get_localhost_origins()
        }
    }
}

/// Get secure default origins (localhost-only).
///
/// Used when no config is provided or config fetch fails.
fn get_localhost_origins() -> Vec<String> {
    vec![
        "http://localhost".to_string(),
        "https://localhost".to_string(),
        "http://127.0.0.1".to_string(),
        "https://127.0.0.1".to_string(),
        "http://[::1]".to_string(),
        "https://[::1]".to_string(),
    ]
}

/// Validate MCP protocol version.
///
/// Per MCP spec: Server must respond with 400 Bad Request if version is invalid/unsupported.
/// Supported versions: 2025-06-18, 2025-03-26, 2024-11-05
fn is_supported_protocol_version(version: &str) -> bool {
    matches!(version, "2025-06-18" | "2025-03-26" | "2024-11-05")
}

/// Send an error response after handler panic.
///
/// This attempts to recover gracefully by sending a JSON-RPC error.
/// It handles all possible message states:
/// - NotStarted: Start a new message and write error
/// - InProgress: Continue the message and write error
/// - Finished: Cannot send error (message already complete)
fn send_panic_error_response() {
    const ERROR_JSON: &[u8] = br#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Internal error: handler panicked"},"id":null}"#;

    MESSAGE_STATE.with(|state| {
        let current = *state.borrow();

        OUTPUT_STREAM.with(|stream_cell| {
            let stream_opt = stream_cell.borrow();
            if let Some(stream) = stream_opt.as_ref() {
                match current {
                    MessageState::NotStarted => {
                        // Start message, write error, finish
                        let _ = write_with_backpressure(stream, b"data: ");
                        let _ = write_with_backpressure(stream, ERROR_JSON);
                        let _ = write_with_backpressure(stream, b"\n\n");
                        let _ = stream.flush();
                        *state.borrow_mut() = MessageState::Finished;
                    }
                    MessageState::InProgress => {
                        // Message started but not finished - write error and finish
                        let _ = write_with_backpressure(stream, ERROR_JSON);
                        let _ = write_with_backpressure(stream, b"\n\n");
                        let _ = stream.flush();
                        *state.borrow_mut() = MessageState::Finished;
                    }
                    MessageState::Finished => {
                        // Message already finished - cannot send error
                    }
                }
            }
        });
    });
}

bindings::export!(Component with_types_in bindings);
