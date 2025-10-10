//! Stdio transport component for the Model Context Protocol (MCP)
//!
//! This component provides stdio-based transport for MCP, handling communication
//! over standard input and output streams with newline-delimited JSON-RPC messages.
//!
//! ## Architecture
//!
//! The transport implements the `wasmcp:mcp/transport` world by:
//! - Exporting `context` interface for message-scoped state management
//! - Exporting `output` interface for newline-delimited message writing
//! - Importing `message-handler` to delegate parsed messages to handler chains
//!
//! State is managed per-message using thread-local storage, which is safe in the
//! single-threaded WASI execution model.
//!
//! ## Message Flow
//!
//! 1. Read newline-delimited JSON-RPC message from stdin
//! 2. Parse bytes into `mcp-message` variant
//! 3. Per-message state is initialized (output stream, context KV store)
//! 4. Session ID is stored in context (stdio has implicit single session)
//! 5. Message is forwarded to handler chain via `message-handler::handle()`
//! 6. Handlers access state through imported `context` and `output` functions
//! 7. These function calls route back to this transport's exported implementations
//! 8. Response is written with newline delimiter and state is cleaned up
//! 9. Loop continues until stdin is closed

use std::cell::RefCell;
use std::collections::HashMap;

mod bindings {
    wit_bindgen::generate!({
        world: "stdio-transport",
        generate_all,
    });
}

use bindings::exports::wasi::cli::run::Guest;
use bindings::exports::wasmcp::mcp::context::Guest as ContextGuest;
use bindings::exports::wasmcp::mcp::output::{Guest as OutputGuest, IoError};
use bindings::wasi::cli::{stderr, stdin, stdout};
use bindings::wasi::io::streams::{InputStream, OutputStream, StreamError};
use bindings::wasmcp::mcp::message_handler::handle;
use bindings::wasmcp::mcp::protocol::{self as mcp, McpMessage, ServerCapability};

use serde_json::Value;

// Per-message state stored in thread-local storage.
// This is safe because WASI components are single-threaded and each message
// is processed sequentially.
thread_local! {
    /// The output stream for the current message's response
    static OUTPUT_STREAM: RefCell<Option<OutputStream>> = const { RefCell::new(None) };

    /// Key-value context storage accessible to handlers via context::get/set
    static CONTEXT_STORE: RefCell<HashMap<String, Vec<u8>>> = RefCell::new(HashMap::new());

    /// Registered server capabilities for this message
    static CAPABILITIES: RefCell<Vec<ServerCapability>> = const { RefCell::new(Vec::new()) };

    /// Track message lifecycle state for output interface
    static MESSAGE_STATE: RefCell<MessageState> = const { RefCell::new(MessageState::NotStarted) };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageState {
    NotStarted,
    InProgress,
    Finished,
}

struct Component;

impl Guest for Component {
    fn run() -> Result<(), ()> {
        // Generate a session ID for this stdio connection
        // For stdio, there's implicitly a single persistent session
        let session_id = "stdio-session";

        // Get the streams once
        let input_stream = stdin::get_stdin();
        let error_stream = stderr::get_stderr();

        // Log startup
        let startup_msg =
            format!("[stdio-transport] Starting MCP server (session: {session_id})\n");
        let _ = error_stream.blocking_write_and_flush(startup_msg.as_bytes());

        // MCP over stdio is a persistent connection that handles multiple messages
        loop {
            // Read a line from stdin (newline-delimited JSON-RPC)
            let line = match read_line(&input_stream) {
                Ok(line) => line,
                Err(StreamError::Closed) => {
                    // stdin closed - this is normal shutdown
                    let shutdown_msg = "[stdio-transport] Stdin closed, shutting down\n";
                    let _ = error_stream.blocking_write_and_flush(shutdown_msg.as_bytes());
                    return Ok(());
                }
                Err(e) => {
                    // Other error - log and continue (might be temporary)
                    let error_msg = format!("[stdio-transport] Error reading from stdin: {e:?}\n");
                    let _ = error_stream.blocking_write_and_flush(error_msg.as_bytes());
                    continue;
                }
            };

            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            // Parse the line as a JSON-RPC message
            let message = match parse_mcp_message(&line) {
                Ok(msg) => msg,
                Err(e) => {
                    // Log parse error to stderr
                    let error_msg = format!("[stdio-transport] Failed to parse JSON-RPC: {e}\n");
                    let _ = error_stream.blocking_write_and_flush(error_msg.as_bytes());

                    // Send JSON-RPC parse error to stdout
                    let output_stream = stdout::get_stdout();
                    let parse_error = format!(
                        "{{\"jsonrpc\":\"2.0\",\"error\":{{\"code\":-32700,\"message\":\"Parse error: {}\"}},\"id\":null}}\n",
                        escape_json_string(&e)
                    );
                    let _ = output_stream.blocking_write_and_flush(parse_error.as_bytes());
                    continue;
                }
            };

            // Determine if we need to send a response (requests do, notifications don't)
            let needs_response = matches!(message, McpMessage::Request(_));

            if needs_response {
                // Get output stream and store in thread-local
                let output_stream = stdout::get_stdout();

                // Initialize per-message state
                OUTPUT_STREAM.with(|s| {
                    *s.borrow_mut() = Some(output_stream);
                });
                MESSAGE_STATE.with(|s| {
                    *s.borrow_mut() = MessageState::NotStarted;
                });
            }

            // Always initialize context storage (handlers may use it)
            CONTEXT_STORE.with(|s| {
                s.borrow_mut().clear();
            });
            CAPABILITIES.with(|c| {
                c.borrow_mut().clear();
            });

            // Store session ID in context
            CONTEXT_STORE.with(|store| {
                store
                    .borrow_mut()
                    .insert("session-id".to_string(), session_id.as_bytes().to_vec());
            });

            // Forward to handler chain with panic recovery
            let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                handle(&message);
            }));

            // If handler panicked and we need a response, send error
            if panic_result.is_err() && needs_response {
                send_panic_error_response();
            }

            // Always clean up state, even after panic
            if needs_response {
                OUTPUT_STREAM.with(|s| {
                    *s.borrow_mut() = None;
                });
                MESSAGE_STATE.with(|s| {
                    *s.borrow_mut() = MessageState::NotStarted;
                });
            }
        }
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

        // Write to output stream
        OUTPUT_STREAM.with(|stream_cell| {
            let stream_opt = stream_cell.borrow();
            let stream = stream_opt
                .as_ref()
                .ok_or_else(|| IoError::Unexpected("No output stream available".to_string()))?;

            write_with_backpressure(stream, &contents)
                .map_err(|e| IoError::Io(format!("Stream error: {e:?}")))
        })
    }

    fn finish_message() -> Result<(), IoError> {
        // Check state
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

        // Write newline delimiter and flush
        OUTPUT_STREAM.with(|stream_cell| {
            let stream_opt = stream_cell.borrow();
            let stream = stream_opt
                .as_ref()
                .ok_or_else(|| IoError::Unexpected("No output stream available".to_string()))?;

            write_with_backpressure(stream, b"\n")
                .and_then(|_| stream.flush())
                .map_err(|e| IoError::Io(format!("Stream error: {e:?}")))
        })
    }
}

// === Stdio I/O ===

/// Read a line from the input stream (reads until newline or EOF)
fn read_line(stream: &InputStream) -> Result<Vec<u8>, StreamError> {
    let mut line = Vec::new();

    loop {
        // Read one byte at a time to detect newlines
        let bytes = stream.blocking_read(1)?;

        if bytes.is_empty() {
            // End of stream
            if line.is_empty() {
                return Err(StreamError::Closed);
            } else {
                // Return what we have (line without newline at EOF)
                return Ok(line);
            }
        }

        let byte = bytes[0];

        if byte == b'\n' {
            // Found newline - return the line (without the newline)
            return Ok(line);
        }

        if byte == b'\r' {
            // Skip carriage returns (handle both \n and \r\n line endings)
            continue;
        }

        // Add byte to line
        line.push(byte);
    }
}

/// Write bytes to stream with proper backpressure handling.
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

// === JSON Utilities ===

/// Escape a string for JSON.
fn escape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());

    for c in s.chars() {
        match c {
            '"' => result.push_str(r#"\""#),
            '\\' => result.push_str(r#"\\"#),
            '\n' => result.push_str(r#"\n"#),
            '\r' => result.push_str(r#"\r"#),
            '\t' => result.push_str(r#"\t"#),
            '\u{0008}' => result.push_str(r#"\b"#),
            '\u{000C}' => result.push_str(r#"\f"#),
            c if c.is_control() => {
                result.push_str(&format!(r#"\u{:04x}"#, c as u32));
            }
            c => result.push(c),
        }
    }

    result
}

/// Send an error response after handler panic.
///
/// This attempts to recover gracefully by sending a JSON-RPC error.
/// It handles all possible message states:
/// - NotStarted: Write complete error message
/// - InProgress: Complete the message with error
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
                        // Write complete error message with newline delimiter
                        let _ = write_with_backpressure(stream, ERROR_JSON);
                        let _ = write_with_backpressure(stream, b"\n");
                        let _ = stream.flush();
                        *state.borrow_mut() = MessageState::Finished;
                    }
                    MessageState::InProgress => {
                        // Message started but not finished - complete it with error
                        let _ = write_with_backpressure(stream, ERROR_JSON);
                        let _ = write_with_backpressure(stream, b"\n");
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
