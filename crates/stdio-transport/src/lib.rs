//! Stateless stdio transport for the Model Context Protocol (MCP)
//!
//! This transport implements the stdio protocol per MCP spec 2025-06-18.
//! It handles JSON-RPC framing over stdin/stdout with newline delimiters.
//!
//! Architecture:
//! - WASI CLI run interface (continuous loop)
//! - Reads newline-delimited JSON from stdin
//! - Delegates to imported server-handler component
//! - Writes newline-delimited JSON to stdout
//! - Stateless: No session management (sessions are optional per world.wit)

#![allow(warnings)]

mod bindings {
    wit_bindgen::generate!({
        world: "stdio-transport",
        generate_all,
    });
}

mod parser;
mod serializer;
mod stream_reader;

use bindings::exports::wasi::cli::run::Guest;
use bindings::wasi::cli::stderr::get_stderr;
use bindings::wasi::cli::stdin::get_stdin;
use bindings::wasi::cli::stdout::get_stdout;
use bindings::wasi::io::streams::{InputStream, OutputStream, StreamError};
use bindings::wasmcp::mcp_v20250618::mcp::{
    ClientNotification, ClientRequest, ClientResult, RequestId, ServerResult,
};
use bindings::wasmcp::mcp_v20250618::server_handler::{
    handle_error, handle_notification, handle_request, handle_result, ErrorCtx, NotificationCtx,
    RequestCtx, ResultCtx,
};

struct StdioTransport;

impl Guest for StdioTransport {
    fn run() -> Result<(), ()> {
        // Get stdin stream (only once, reused for reading)
        let stdin = get_stdin();

        // Main message loop
        loop {
            // Get fresh stdout for each message (ClientNotifier takes ownership)
            let stdout = get_stdout();
            let stderr = get_stderr();

            match process_one_message(&stdin, stdout, &stderr) {
                Ok(()) => continue,
                Err(e) if e == "EOF" => {
                    // Clean shutdown on EOF
                    return Ok(());
                }
                Err(e) => {
                    // Log error to stderr and continue
                    let stderr = get_stderr();
                    let _ = write_stderr(&stderr, &format!("Error: {}\n", e));
                    continue;
                }
            }
        }
    }
}

/// Process one JSON-RPC message from stdin
fn process_one_message(
    stdin: &InputStream,
    stdout: OutputStream, // Takes ownership
    stderr: &OutputStream,
) -> Result<(), String> {
    // Read one line from stdin
    let line = read_line(stdin)?;

    // Parse JSON
    let json_rpc: serde_json::Value =
        serde_json::from_slice(&line).map_err(|e| format!("Invalid JSON: {}", e))?;

    // Determine message type
    if json_rpc.get("method").is_some() {
        // It's a request or notification
        if let Some(id) = json_rpc.get("id") {
            // Request - handle and respond
            handle_json_rpc_request(&json_rpc, id, stdout, stderr)
        } else {
            // Notification - handle but don't respond
            handle_json_rpc_notification(&json_rpc, stdout, stderr)
        }
    } else if json_rpc.get("result").is_some() || json_rpc.get("error").is_some() {
        // It's a response from client to server - just process it
        handle_json_rpc_response(&json_rpc, stdout, stderr)
    } else {
        Err("Invalid JSON-RPC message: no method, result, or error field".to_string())
    }
}

fn handle_json_rpc_request(
    json_rpc: &serde_json::Value,
    id: &serde_json::Value,
    stdout: OutputStream, // Takes ownership
    _stderr: &OutputStream,
) -> Result<(), String> {
    // Parse request ID
    let request_id = parser::parse_request_id(id)?;

    // Check if this is a transport-level method
    let method = json_rpc
        .get("method")
        .and_then(|m| m.as_str())
        .ok_or("Missing method field")?;

    // Handle transport-level methods directly
    match method {
        "initialize" => return handle_initialize_request(json_rpc, request_id, &stdout),
        "ping" => return handle_ping_request(request_id, &stdout),
        "logging/setLevel" => return handle_set_level_request(request_id, &stdout),
        _ => {
            // Delegate all other requests to server-handler
        }
    }

    // Parse client request from JSON
    let client_request = parser::parse_client_request(json_rpc)?;

    // Create context (stateless: no session, no JWT)
    // Stdio transport: use latest protocol version (spec only defines default for HTTP)
    let ctx = RequestCtx {
        request_id: request_id.clone(),
        jwt: None,
        session_id: None,
        message_stream: Some(&stdout),
        protocol_version: "2025-06-18".to_string(),
    };

    // Delegate to server-handler (may send notifications via output stream)
    let result = handle_request(&ctx, &client_request);

    // Write final JSON-RPC response to stdout
    write_json_rpc_response(&stdout, request_id, result)?;

    Ok(())
}

fn handle_initialize_request(
    json_rpc: &serde_json::Value,
    request_id: RequestId,
    stdout: &OutputStream,
) -> Result<(), String> {
    use bindings::wasmcp::mcp_v20250618::mcp::{
        ClientRequest, ListPromptsRequest, ListResourcesRequest, ListToolsRequest, ProtocolVersion,
    };

    // Parse initialize request parameters
    let params = json_rpc
        .get("params")
        .ok_or("Missing params in initialize request")?;

    let client_protocol_version = params
        .get("protocolVersion")
        .and_then(|v| v.as_str())
        .ok_or("Missing protocolVersion in initialize params")?;

    // Negotiate protocol version
    let protocol_version = match client_protocol_version {
        "2025-06-18" => ProtocolVersion::V20250618,
        "2025-03-26" => ProtocolVersion::V20250326,
        "2024-11-05" => ProtocolVersion::V20241105,
        _ => {
            // Client sent unsupported version, respond with our latest
            ProtocolVersion::V20250618
        }
    };

    // Discover capabilities by calling downstream handler's list methods
    let capabilities = discover_capabilities();

    // Serialize capabilities
    let capabilities_json = serialize_capabilities(&capabilities);

    // Build server info
    let server_name = "wasmcp-stdio-transport";
    let server_version = env!("CARGO_PKG_VERSION");

    // Write initialize result
    let json_result = serde_json::json!({
        "jsonrpc": "2.0",
        "id": match &request_id {
            RequestId::Number(n) => serde_json::Value::Number(serde_json::Number::from(*n)),
            RequestId::String(s) => serde_json::Value::String(s.clone()),
        },
        "result": {
            "protocolVersion": match protocol_version {
                ProtocolVersion::V20250618 => "2025-06-18",
                ProtocolVersion::V20250326 => "2025-03-26",
                ProtocolVersion::V20241105 => "2024-11-05",
            },
            "capabilities": capabilities_json,
            "serverInfo": {
                "name": server_name,
                "title": "WASMCP stdio Transport",
                "version": server_version,
            }
        }
    });

    let mut json_str = serde_json::to_string(&json_result)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    json_str.push('\n');

    write_chunked(stdout, json_str.as_bytes())
}

fn handle_ping_request(request_id: RequestId, stdout: &OutputStream) -> Result<(), String> {
    // Ping is a no-op - just return empty success
    let json_result = serde_json::json!({
        "jsonrpc": "2.0",
        "id": match &request_id {
            RequestId::Number(n) => serde_json::Value::Number(serde_json::Number::from(*n)),
            RequestId::String(s) => serde_json::Value::String(s.clone()),
        },
        "result": {}
    });

    let mut json_str = serde_json::to_string(&json_result)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    json_str.push('\n');

    write_chunked(stdout, json_str.as_bytes())
}

fn handle_set_level_request(request_id: RequestId, stdout: &OutputStream) -> Result<(), String> {
    // logging/setLevel is a no-op in stateless transport
    let json_result = serde_json::json!({
        "jsonrpc": "2.0",
        "id": match &request_id {
            RequestId::Number(n) => serde_json::Value::Number(serde_json::Number::from(*n)),
            RequestId::String(s) => serde_json::Value::String(s.clone()),
        },
        "result": {}
    });

    let mut json_str = serde_json::to_string(&json_result)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    json_str.push('\n');

    write_chunked(stdout, json_str.as_bytes())
}

fn discover_capabilities() -> bindings::wasmcp::mcp_v20250618::mcp::ServerCapabilities {
    use bindings::wasmcp::mcp_v20250618::mcp::{
        ClientRequest, CompleteRequest, CompletionArgument, CompletionPromptReference,
        CompletionReference, ListPromptsRequest, ListResourcesRequest, ListToolsRequest,
        ServerCapabilities, ServerLists, ServerResult,
    };

    // Try to discover what the downstream handler supports by calling list methods
    let mut list_flags = ServerLists::empty();

    // Create context for discovery calls
    let ctx = RequestCtx {
        request_id: RequestId::Number(0),
        jwt: None,
        session_id: None,
        message_stream: None,
        protocol_version: "2025-06-18".to_string(),
    };

    // Try list-tools
    let req = ClientRequest::ToolsList(ListToolsRequest { cursor: None });
    if let Ok(_) = handle_request(&ctx, &req) {
        list_flags |= ServerLists::TOOLS;
    }

    // Try list-resources
    let req = ClientRequest::ResourcesList(ListResourcesRequest { cursor: None });
    if let Ok(_) = handle_request(&ctx, &req) {
        list_flags |= ServerLists::RESOURCES;
    }

    // Try list-prompts and use result to test completions
    let mut has_completions = false;
    let req = ClientRequest::PromptsList(ListPromptsRequest { cursor: None });
    if let Ok(ServerResult::PromptsList(prompts_result)) = handle_request(&ctx, &req) {
        list_flags |= ServerLists::PROMPTS;

        // Try to discover completions support using a real prompt
        if !prompts_result.prompts.is_empty() {
            let first_prompt = &prompts_result.prompts[0];

            // Check if prompt has arguments to complete
            if let Some(ref options) = first_prompt.options {
                if let Some(ref args) = options.arguments {
                    if !args.is_empty() {
                        // Try completion with real prompt name and first argument
                        let completion_request = CompleteRequest {
                            argument: CompletionArgument {
                                name: args[0].name.clone(),
                                value: "".to_string(),
                            },
                            ref_: CompletionReference::Prompt(CompletionPromptReference {
                                name: first_prompt.name.clone(),
                                title: None,
                            }),
                            context: None,
                        };

                        // Test if completions are supported
                        let req = ClientRequest::CompletionComplete(completion_request);
                        match handle_request(&ctx, &req) {
                            Ok(_) => has_completions = true,
                            Err(
                                bindings::wasmcp::mcp_v20250618::mcp::ErrorCode::MethodNotFound(_),
                            ) => {
                                has_completions = false;
                            }
                            Err(_) => {
                                // Other errors (InvalidParams, etc.) suggest completions might be
                                // supported but our test failed - assume supported
                                has_completions = true;
                            }
                        }
                    }
                }
            }
        }
    }

    ServerCapabilities {
        completions: if has_completions {
            Some("{}".to_string())
        } else {
            None
        },
        experimental: None,
        logging: Some("{}".to_string()),
        list_changed: if !list_flags.is_empty() {
            Some(list_flags)
        } else {
            None
        },
        subscriptions: None,
    }
}

fn serialize_capabilities(
    caps: &bindings::wasmcp::mcp_v20250618::mcp::ServerCapabilities,
) -> serde_json::Value {
    let mut result = serde_json::Map::new();

    if let Some(ref _completions) = caps.completions {
        result.insert("completions".to_string(), serde_json::json!({}));
    }

    if let Some(ref _logging) = caps.logging {
        result.insert("logging".to_string(), serde_json::json!({}));
    }

    // Serialize list_changed capabilities - each capability type gets its own nested object
    if let Some(flags) = caps.list_changed {
        if flags.contains(bindings::wasmcp::mcp_v20250618::mcp::ServerLists::TOOLS) {
            let mut tools_caps = serde_json::Map::new();
            tools_caps.insert("listChanged".to_string(), serde_json::json!(true));
            result.insert("tools".to_string(), serde_json::Value::Object(tools_caps));
        }
        if flags.contains(bindings::wasmcp::mcp_v20250618::mcp::ServerLists::RESOURCES) {
            let mut resources_caps = serde_json::Map::new();
            resources_caps.insert("listChanged".to_string(), serde_json::json!(true));
            result.insert(
                "resources".to_string(),
                serde_json::Value::Object(resources_caps),
            );
        }
        if flags.contains(bindings::wasmcp::mcp_v20250618::mcp::ServerLists::PROMPTS) {
            let mut prompts_caps = serde_json::Map::new();
            prompts_caps.insert("listChanged".to_string(), serde_json::json!(true));
            result.insert(
                "prompts".to_string(),
                serde_json::Value::Object(prompts_caps),
            );
        }
    }

    if let Some(ref _subscriptions) = caps.subscriptions {
        // Handle subscriptions if present
        let mut resources_caps = result
            .get_mut("resources")
            .and_then(|v| v.as_object_mut())
            .map(|o| o.clone())
            .unwrap_or_else(|| serde_json::Map::new());
        resources_caps.insert("subscribe".to_string(), serde_json::json!(true));
        result.insert(
            "resources".to_string(),
            serde_json::Value::Object(resources_caps),
        );
    }

    serde_json::Value::Object(result)
}

fn handle_json_rpc_notification(
    json_rpc: &serde_json::Value,
    _stdout: OutputStream, // Takes ownership but unused
    _stderr: &OutputStream,
) -> Result<(), String> {
    // Parse notification from JSON
    let notification = parser::parse_client_notification(json_rpc)?;

    // Create context (stateless: no JWT, no session)
    // Stdio transport: use latest protocol version (spec only defines default for HTTP)
    let ctx = NotificationCtx {
        jwt: None,
        session_id: None,
        protocol_version: "2025-06-18".to_string(),
    };

    // Forward to server-handler (no response expected)
    handle_notification(&ctx, &notification);

    Ok(())
}

fn handle_json_rpc_response(
    json_rpc: &serde_json::Value,
    _stdout: OutputStream, // Takes ownership but unused
    _stderr: &OutputStream,
) -> Result<(), String> {
    // Parse response ID (required for responses)
    let id = json_rpc.get("id").ok_or("Missing id in response")?;
    let request_id = parser::parse_request_id(id)?;

    // Parse client response from JSON
    let response_result = parser::parse_client_response(json_rpc)?;

    // Forward to server-handler (no response expected)
    // Split result vs error handling per new interface
    match response_result {
        Ok(client_result) => {
            // Create result context
            // Stdio transport: use latest protocol version (spec only defines default for HTTP)
            let ctx = ResultCtx {
                request_id,
                jwt: None,
                session_id: None,
                protocol_version: "2025-06-18".to_string(),
            };
            handle_result(&ctx, client_result);
        }
        Err(error_code) => {
            // Create error context
            // Stdio transport: use latest protocol version (spec only defines default for HTTP)
            let ctx = ErrorCtx {
                request_id: Some(request_id),
                jwt: None,
                session_id: None,
                protocol_version: "2025-06-18".to_string(),
            };
            handle_error(&ctx, &error_code);
        }
    }

    Ok(())
}

/// Read one line from stdin (up to newline, excluding the newline)
fn read_line(stdin: &InputStream) -> Result<Vec<u8>, String> {
    let mut buffer = Vec::new();

    loop {
        match stdin.blocking_read(4096) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    // EOF - return what we have or error if empty
                    if buffer.is_empty() {
                        return Err("EOF".to_string());
                    } else {
                        // Return partial line on EOF (no trailing newline)
                        return Ok(buffer);
                    }
                }

                // Look for newline in chunk
                for byte in chunk {
                    if byte == b'\n' {
                        // Found newline - return line without it
                        return Ok(buffer);
                    }
                    buffer.push(byte);
                }
            }
            Err(StreamError::Closed) => {
                // Stream closed
                if buffer.is_empty() {
                    return Err("EOF".to_string());
                } else {
                    return Ok(buffer);
                }
            }
            Err(e) => {
                return Err(format!("Stream error reading stdin: {:?}", e));
            }
        }
    }
}

/// Write a JSON-RPC response to stdout (with trailing newline)
fn write_json_rpc_response(
    stdout: &OutputStream,
    request_id: RequestId,
    result: Result<ServerResult, bindings::wasmcp::mcp_v20250618::mcp::ErrorCode>,
) -> Result<(), String> {
    // Serialize to JSON-RPC
    let json_rpc =
        serializer::serialize_jsonrpc_response(&request_id, result.as_ref().map_err(|e| e));

    // Convert to string with trailing newline
    let mut json_str =
        serde_json::to_string(&json_rpc).map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    json_str.push('\n');

    // Write in 4096-byte chunks per WASI spec
    write_chunked(stdout, json_str.as_bytes())
}

/// Write bytes to output stream in 4096-byte chunks
fn write_chunked(stream: &OutputStream, bytes: &[u8]) -> Result<(), String> {
    let mut offset = 0;
    const MAX_WRITE: usize = 4096;

    while offset < bytes.len() {
        let chunk_size = (bytes.len() - offset).min(MAX_WRITE);
        let chunk = &bytes[offset..offset + chunk_size];

        stream
            .blocking_write_and_flush(chunk)
            .map_err(|e| match e {
                StreamError::LastOperationFailed(_) => "Stream write failed".to_string(),
                StreamError::Closed => "Stream closed".to_string(),
            })?;

        offset += chunk_size;
    }

    Ok(())
}

/// Write a UTF-8 string to stderr for logging
fn write_stderr(stderr: &OutputStream, message: &str) -> Result<(), String> {
    write_chunked(stderr, message.as_bytes())
}

bindings::export!(StdioTransport with_types_in bindings);
