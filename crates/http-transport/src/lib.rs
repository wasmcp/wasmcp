//! Stateless HTTP transport for the Model Context Protocol (MCP)
//!
//! This transport implements the Streamable HTTP protocol per MCP spec 2025-06-18.
//! It handles JSON-RPC framing, SSE responses, and Origin validation.
//!
//! Architecture:
//! - WASI HTTP proxy interface (incoming requests)
//! - Delegates to imported server-handler component
//! - Returns SSE streams for all responses
//! - Stateless: No session management (sessions are optional per world.wit)

#![allow(warnings)]

mod bindings {
    wit_bindgen::generate!({
        world: "http-transport",
        generate_all,
    });
}

mod parser;
mod serializer;
mod stream_reader;

use bindings::exports::wasi::http::incoming_handler::Guest;
use bindings::wasi::cli::environment::get_environment;
use bindings::wasi::http::types::{
    Fields, IncomingRequest, OutgoingBody, OutgoingResponse, ResponseOutparam,
};
use bindings::wasi::io::streams::OutputStream;
use bindings::wasmcp::protocol::mcp::{
    ClientNotification, ClientRequest, ClientResponse, RequestId, ServerResponse,
};
use bindings::wasmcp::protocol::server_messages::Context;
use bindings::wasmcp::server::handler::{handle_notification, handle_request, handle_response};

struct HttpTransport;

impl Guest for HttpTransport {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        match handle_http_request(request) {
            Ok(response) => {
                ResponseOutparam::set(response_out, Ok(response));
            }
            Err(e) => {
                // Return error response
                let response = create_error_response(e);
                ResponseOutparam::set(response_out, Ok(response));
            }
        }
    }
}

fn handle_http_request(request: IncomingRequest) -> Result<OutgoingResponse, String> {
    // 1. Validate Origin header (DNS rebinding protection)
    validate_origin(&request)?;

    // 2. Validate MCP-Protocol-Version header (if present)
    validate_protocol_version(&request)?;

    // 3. Parse method and handle accordingly
    let method = request.method();

    match method {
        bindings::wasi::http::types::Method::Post => handle_post(request),
        bindings::wasi::http::types::Method::Get => handle_get(request),
        bindings::wasi::http::types::Method::Delete => {
            // DELETE not supported in stateless mode
            create_method_not_allowed_response()
        }
        _ => create_method_not_allowed_response(),
    }
}

fn handle_post(request: IncomingRequest) -> Result<OutgoingResponse, String> {
    // Validate Accept header per spec
    // Per MCP spec: "The client MUST include an Accept header, listing both application/json
    // and text/event-stream as supported content types"
    validate_accept_header(&request)?;

    // Read request body
    let body = read_request_body(request.consume().map_err(|_| "Failed to consume request")?)?;

    // Parse JSON-RPC message
    let json_rpc: serde_json::Value =
        serde_json::from_slice(&body).map_err(|e| format!("Invalid JSON: {}", e))?;

    // Determine message type (request, notification, or response)
    if json_rpc.get("method").is_some() {
        // It's a request or notification
        if let Some(id) = json_rpc.get("id") {
            // Request - handle and return SSE stream
            handle_json_rpc_request(&json_rpc, id)
        } else {
            // Notification - accept and return 202
            handle_json_rpc_notification(&json_rpc)
        }
    } else if json_rpc.get("result").is_some() || json_rpc.get("error").is_some() {
        // It's a response (from client to server)
        handle_json_rpc_response(&json_rpc)
    } else {
        Err("Invalid JSON-RPC message".to_string())
    }
}

fn handle_get(_request: IncomingRequest) -> Result<OutgoingResponse, String> {
    // In stateless mode, we don't support GET (no persistent SSE streams)
    // The spec allows servers to return 405 Method Not Allowed
    create_method_not_allowed_response()
}

fn handle_json_rpc_request(
    json_rpc: &serde_json::Value,
    id: &serde_json::Value,
) -> Result<OutgoingResponse, String> {
    // Parse request ID
    let request_id = parse_request_id(id)?;

    // Check if this is a transport-level method
    let method = json_rpc
        .get("method")
        .and_then(|m| m.as_str())
        .ok_or("Missing method field")?;

    // Handle transport-level methods directly
    match method {
        "initialize" => return handle_initialize_request(json_rpc, request_id),
        "ping" => return handle_ping_request(request_id),
        "logging/setLevel" => return handle_set_level_request(request_id),
        _ => {
            // Delegate all other requests to server-handler
        }
    }

    // Parse client request from JSON
    let client_request = parse_client_request(json_rpc)?;

    // Create headers FIRST
    let headers = Fields::new();
    headers
        .set(
            &"content-type".to_string(),
            &[b"text/event-stream".to_vec()],
        )
        .map_err(|_| "Failed to set content-type")?;
    headers
        .set(&"cache-control".to_string(), &[b"no-cache".to_vec()])
        .map_err(|_| "Failed to set cache-control")?;
    // Note: Transfer-Encoding is managed by the WASI HTTP runtime, don't set it manually

    // Create response with headers
    let response = OutgoingResponse::new(headers);
    response
        .set_status_code(200)
        .map_err(|_| "Failed to set status")?;

    // Get body and output stream
    let body = response.body().map_err(|_| "Failed to get response body")?;
    let output_stream = body.write().map_err(|_| "Failed to get output stream")?;

    // Create context (stateless: no session, no claims)
    let ctx = Context {
        claims: None,
        session_id: None,
        data: vec![],
    };

    // Delegate to server-handler (may send notifications via output stream)
    let result = handle_request(&ctx, (&client_request, &request_id), Some(&output_stream));

    // Write final JSON-RPC response to SSE stream
    write_sse_response(&output_stream, request_id, result)?;

    // Drop output_stream to finalize it
    drop(output_stream);

    // Finish the body to finalize the stream
    OutgoingBody::finish(body, None).map_err(|_| "Failed to finish body")?;

    Ok(response)
}

fn handle_initialize_request(
    json_rpc: &serde_json::Value,
    request_id: RequestId,
) -> Result<OutgoingResponse, String> {
    use bindings::wasmcp::protocol::mcp::{
        ClientCapabilities, Implementation, InitializeRequest, InitializeResult, ProtocolVersion,
        ServerCapabilities,
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

    // Serialize capabilities before we move anything
    let capabilities_json = serialize_capabilities(&capabilities);

    // Build server info
    let server_name = "wasmcp-http-transport".to_string();
    let server_title = Some("wasmcp HTTP Transport".to_string());
    let server_version = env!("CARGO_PKG_VERSION").to_string();

    // Write JSON response (not SSE - no notifier, no events)
    let headers = Fields::new();
    headers
        .set(&"content-type".to_string(), &[b"application/json".to_vec()])
        .map_err(|_| "Failed to set content-type")?;
    let response = OutgoingResponse::new(headers);
    response
        .set_status_code(200)
        .map_err(|_| "Failed to set status")?;

    let body = response.body().map_err(|_| "Failed to get response body")?;
    let output_stream = body.write().map_err(|_| "Failed to get output stream")?;

    // Write initialize result as plain JSON-RPC response
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
                "title": server_title,
                "version": server_version,
            }
        }
    });

    let json_str = serde_json::to_string(&json_result)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    write_chunked(&output_stream, json_str.as_bytes())?;

    drop(output_stream);
    OutgoingBody::finish(body, None).map_err(|_| "Failed to finish body")?;

    Ok(response)
}

fn handle_ping_request(request_id: RequestId) -> Result<OutgoingResponse, String> {
    // Ping is a no-op - just return empty success as plain JSON
    let headers = Fields::new();
    headers
        .set(&"content-type".to_string(), &[b"application/json".to_vec()])
        .map_err(|_| "Failed to set content-type")?;
    let response = OutgoingResponse::new(headers);
    response
        .set_status_code(200)
        .map_err(|_| "Failed to set status")?;

    let body = response.body().map_err(|_| "Failed to get response body")?;
    let output_stream = body.write().map_err(|_| "Failed to get output stream")?;

    // Return empty result object
    let json_result = serde_json::json!({
        "jsonrpc": "2.0",
        "id": match &request_id {
            RequestId::Number(n) => serde_json::Value::Number(serde_json::Number::from(*n)),
            RequestId::String(s) => serde_json::Value::String(s.clone()),
        },
        "result": {}
    });

    let json_str = serde_json::to_string(&json_result)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    write_chunked(&output_stream, json_str.as_bytes())?;

    drop(output_stream);
    OutgoingBody::finish(body, None).map_err(|_| "Failed to finish body")?;

    Ok(response)
}

fn handle_set_level_request(request_id: RequestId) -> Result<OutgoingResponse, String> {
    // logging/setLevel is a no-op in stateless transport as plain JSON
    // We can't maintain logging level state across requests
    let headers = Fields::new();
    headers
        .set(&"content-type".to_string(), &[b"application/json".to_vec()])
        .map_err(|_| "Failed to set content-type")?;
    let response = OutgoingResponse::new(headers);
    response
        .set_status_code(200)
        .map_err(|_| "Failed to set status")?;

    let body = response.body().map_err(|_| "Failed to get response body")?;
    let output_stream = body.write().map_err(|_| "Failed to get output stream")?;

    // Return empty result object
    let json_result = serde_json::json!({
        "jsonrpc": "2.0",
        "id": match &request_id {
            RequestId::Number(n) => serde_json::Value::Number(serde_json::Number::from(*n)),
            RequestId::String(s) => serde_json::Value::String(s.clone()),
        },
        "result": {}
    });

    let json_str = serde_json::to_string(&json_result)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    write_chunked(&output_stream, json_str.as_bytes())?;

    drop(output_stream);
    OutgoingBody::finish(body, None).map_err(|_| "Failed to finish body")?;

    Ok(response)
}

fn discover_capabilities() -> bindings::wasmcp::protocol::mcp::ServerCapabilities {
    use bindings::wasmcp::protocol::mcp::{
        ClientRequest, CompleteRequest, CompletionArgument, CompletionPromptReference,
        CompletionReference, ListPromptsRequest, ListResourcesRequest, ListToolsRequest,
        ServerCapabilities, ServerLists, ServerResponse,
    };

    // Try to discover what the downstream handler supports by calling list methods
    // With optional output stream, we can pass None for discovery calls
    let mut list_flags = ServerLists::empty();

    // Create a context for discovery calls
    let ctx = Context {
        claims: None,
        session_id: None,
        data: vec![],
    };

    // Try list-tools
    let req = ClientRequest::ToolsList(ListToolsRequest { cursor: None });
    let id = RequestId::Number(0);
    if let Ok(_) = handle_request(&ctx, (&req, &id), None) {
        list_flags |= ServerLists::TOOLS;
    }

    // Try list-resources
    let req = ClientRequest::ResourcesList(ListResourcesRequest { cursor: None });
    let id = RequestId::Number(1);
    if let Ok(_) = handle_request(&ctx, (&req, &id), None) {
        list_flags |= ServerLists::RESOURCES;
    }

    // Try list-prompts and use result to test completions
    let mut has_completions = false;
    let req = ClientRequest::PromptsList(ListPromptsRequest { cursor: None });
    let id = RequestId::Number(2);
    if let Ok(ServerResponse::PromptsList(prompts_result)) = handle_request(&ctx, (&req, &id), None)
    {
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
                        let id = RequestId::Number(3);
                        match handle_request(&ctx, (&req, &id), None) {
                            Ok(_) => has_completions = true,
                            Err(bindings::wasmcp::protocol::mcp::ErrorCode::MethodNotFound(_)) => {
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
        logging: Some("{}".to_string()), // We provide logging mechanism
        list_changed: if !list_flags.is_empty() {
            Some(list_flags)
        } else {
            None
        },
        subscriptions: None, // No subscribe support in stateless transport
    }
}

fn serialize_capabilities(
    caps: &bindings::wasmcp::protocol::mcp::ServerCapabilities,
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
        if flags.contains(bindings::wasmcp::protocol::mcp::ServerLists::TOOLS) {
            let mut tools_caps = serde_json::Map::new();
            tools_caps.insert("listChanged".to_string(), serde_json::json!(true));
            result.insert("tools".to_string(), serde_json::Value::Object(tools_caps));
        }
        if flags.contains(bindings::wasmcp::protocol::mcp::ServerLists::RESOURCES) {
            let mut resources_caps = serde_json::Map::new();
            resources_caps.insert("listChanged".to_string(), serde_json::json!(true));
            result.insert(
                "resources".to_string(),
                serde_json::Value::Object(resources_caps),
            );
        }
        if flags.contains(bindings::wasmcp::protocol::mcp::ServerLists::PROMPTS) {
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

fn write_chunked(output_stream: &OutputStream, bytes: &[u8]) -> Result<(), String> {
    use bindings::wasi::io::streams::StreamError;

    let mut offset = 0;
    const MAX_WRITE: usize = 4096;

    while offset < bytes.len() {
        let chunk_size = (bytes.len() - offset).min(MAX_WRITE);
        let chunk = &bytes[offset..offset + chunk_size];

        output_stream
            .blocking_write_and_flush(chunk)
            .map_err(|e| match e {
                StreamError::LastOperationFailed(_) => "Stream write failed".to_string(),
                StreamError::Closed => "Stream closed".to_string(),
            })?;

        offset += chunk_size;
    }

    Ok(())
}

fn handle_json_rpc_notification(json_rpc: &serde_json::Value) -> Result<OutgoingResponse, String> {
    // Parse notification from JSON
    let notification = parser::parse_client_notification(json_rpc)?;

    // Create context (stateless)
    let ctx = Context {
        claims: None,
        session_id: None,
        data: vec![],
    };

    // Forward to server-handler (no response expected)
    handle_notification(&ctx, &notification);

    // Return 202 Accepted
    let response = OutgoingResponse::new(Fields::new());
    response
        .set_status_code(202)
        .map_err(|_| "Failed to set status")?;
    Ok(response)
}

fn handle_json_rpc_response(json_rpc: &serde_json::Value) -> Result<OutgoingResponse, String> {
    // Parse response ID (required for responses)
    let id = json_rpc.get("id").ok_or("Missing id in response")?;
    let request_id = parser::parse_request_id(id)?;

    // Parse client response from JSON
    let response_result = parser::parse_client_response(json_rpc)?;

    // Create context (stateless)
    let ctx = Context {
        claims: None,
        session_id: None,
        data: vec![],
    };

    // Convert to Result<(ClientResponse, RequestId), ErrorCode>
    let result_with_id = response_result.map(|resp| (resp, request_id));

    // Forward to server-handler (no response expected)
    handle_response(&ctx, result_with_id);

    // Return 202 Accepted
    let response = OutgoingResponse::new(Fields::new());
    response
        .set_status_code(202)
        .map_err(|_| "Failed to set status")?;
    Ok(response)
}

fn validate_accept_header(request: &IncomingRequest) -> Result<(), String> {
    // Per MCP spec: "The client MUST include an Accept header, listing both application/json
    // and text/event-stream as supported content types"
    // https://modelcontextprotocol.io/specification/2025-06-18/basic/transports#sending-messages-to-the-server

    let headers = request.headers();
    let accept_values = headers.get(&"accept".to_string());

    if accept_values.is_empty() {
        return Err("Missing Accept header".to_string());
    }

    let accept_str = String::from_utf8(accept_values[0].clone())
        .map_err(|_| "Invalid Accept header encoding".to_string())?;

    // Check if both required content types are present
    let has_json = accept_str.contains("application/json") || accept_str.contains("*/*");
    let has_sse = accept_str.contains("text/event-stream") || accept_str.contains("*/*");

    if !has_json || !has_sse {
        return Err(
            "Accept header must include both application/json and text/event-stream".to_string(),
        );
    }

    Ok(())
}

fn validate_protocol_version(request: &IncomingRequest) -> Result<(), String> {
    // Per MCP spec: If using HTTP, the client MUST include the MCP-Protocol-Version header
    // https://modelcontextprotocol.io/specification/2025-06-18/basic/transports#protocol-version-header

    let headers = request.headers();
    let version_values = headers.get(&"mcp-protocol-version".to_string());

    if version_values.is_empty() {
        // No version header - assume 2025-03-26 for backwards compatibility
        // Per spec: "the server SHOULD assume protocol version 2025-03-26"
        return Ok(());
    }

    let version_str = String::from_utf8(version_values[0].clone())
        .map_err(|_| "Invalid MCP-Protocol-Version header encoding".to_string())?;

    // Validate supported versions
    match version_str.as_str() {
        "2025-06-18" | "2025-03-26" | "2024-11-05" => Ok(()),
        _ => Err(format!("Unsupported MCP-Protocol-Version: {}", version_str)),
    }
}

/// Validate Origin header to prevent DNS rebinding attacks
///
/// Per MCP spec: Servers MUST validate the Origin header to prevent DNS rebinding attacks
/// https://modelcontextprotocol.io/specification/2025-06-18/basic/transports#security-warning
///
/// Configuration via environment variables:
/// - MCP_ALLOWED_ORIGINS: Comma-separated list of allowed origins (e.g., "http://localhost:3000,https://app.example.com")
///   - Special value "*" allows all origins (INSECURE - only for development)
/// - MCP_REQUIRE_ORIGIN: "true" to require Origin header, "false" to allow missing Origin (default: false)
///
/// Default behavior: If MCP_ALLOWED_ORIGINS is not set, only localhost origins are allowed.
fn validate_origin(request: &IncomingRequest) -> Result<(), String> {
    // Get the Origin header
    let headers = request.headers();
    let origin_values = headers.get(&"origin".to_string());

    // Get environment variables
    let env_vars = get_environment();
    let require_origin = env_vars
        .iter()
        .find(|(k, _)| k == "MCP_REQUIRE_ORIGIN")
        .map(|(_, v)| v.as_str())
        .unwrap_or("false");

    let allowed_origins = env_vars
        .iter()
        .find(|(k, _)| k == "MCP_ALLOWED_ORIGINS")
        .map(|(_, v)| v.as_str());

    // If no Origin header, check if we require it
    let origin = if origin_values.is_empty() {
        if require_origin == "true" {
            return Err("Origin header required but not provided".to_string());
        }
        // No Origin header but not required - allow (non-browser clients)
        return Ok(());
    } else {
        // Take first Origin value and decode
        String::from_utf8(origin_values[0].clone())
            .map_err(|_| "Invalid Origin header encoding".to_string())?
    };

    // Check allowed origins
    match allowed_origins {
        Some(allowed) => {
            // Comma-separated list of allowed origins
            let allowed_list: Vec<&str> = allowed.split(',').map(|s| s.trim()).collect();

            // Special case: "*" means allow all (INSECURE - development only)
            if allowed_list.contains(&"*") {
                return Ok(());
            }

            // Check if origin is in allowed list
            if allowed_list.contains(&origin.as_str()) {
                Ok(())
            } else {
                Err(format!(
                    "Origin '{}' not in allowed list. Set MCP_ALLOWED_ORIGINS environment variable.",
                    origin
                ))
            }
        }
        None => {
            // No configuration - default to localhost only for security
            validate_localhost_origin(&origin)
        }
    }
}

/// Validate that origin is a localhost origin (default secure behavior)
fn validate_localhost_origin(origin: &str) -> Result<(), String> {
    let localhost_patterns = [
        "http://localhost",
        "https://localhost",
        "http://127.0.0.1",
        "https://127.0.0.1",
        "http://[::1]",
        "https://[::1]",
    ];

    for pattern in &localhost_patterns {
        if origin.starts_with(pattern) {
            return Ok(());
        }
    }

    Err(format!(
        "Origin '{}' not allowed. By default, only localhost origins are permitted. \
        Set MCP_ALLOWED_ORIGINS environment variable to allow other origins.",
        origin
    ))
}

fn read_request_body(body: bindings::wasi::http::types::IncomingBody) -> Result<Vec<u8>, String> {
    use bindings::wasi::io::streams::StreamError;

    let stream = body.stream().map_err(|_| "Failed to get body stream")?;
    let mut buffer = Vec::new();

    loop {
        match stream.blocking_read(8192) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    break;
                }
                buffer.extend_from_slice(&chunk);
            }
            Err(StreamError::Closed) => break,
            Err(e) => {
                drop(stream); // Explicit cleanup before error
                return Err(format!("Stream error: {:?}", e));
            }
        }
    }

    // Explicitly drop stream child resource before parent body is dropped
    drop(stream);

    Ok(buffer)
}

fn parse_request_id(id: &serde_json::Value) -> Result<RequestId, String> {
    parser::parse_request_id(id)
}

fn parse_client_request(json_rpc: &serde_json::Value) -> Result<ClientRequest, String> {
    parser::parse_client_request(json_rpc)
}

fn write_sse_response(
    output_stream: &OutputStream,
    request_id: RequestId,
    result: Result<ServerResponse, bindings::wasmcp::protocol::mcp::ErrorCode>,
) -> Result<(), String> {
    use bindings::wasi::io::streams::StreamError;

    // Serialize to JSON-RPC
    let json_rpc =
        serializer::serialize_jsonrpc_response(&request_id, result.as_ref().map_err(|e| e));

    // Format as SSE event
    let event_data = serializer::format_sse_event(&json_rpc);

    // Write to output stream in chunks (blocking-write-and-flush limited to 4096 bytes per WASI spec)
    let bytes = event_data.as_bytes();
    let mut offset = 0;
    const MAX_WRITE: usize = 4096;

    while offset < bytes.len() {
        let chunk_size = (bytes.len() - offset).min(MAX_WRITE);
        let chunk = &bytes[offset..offset + chunk_size];

        output_stream
            .blocking_write_and_flush(chunk)
            .map_err(|e| match e {
                StreamError::LastOperationFailed(_) => "Stream write failed".to_string(),
                StreamError::Closed => "Stream closed".to_string(),
            })?;

        offset += chunk_size;
    }

    Ok(())
}

fn create_error_response(error: String) -> OutgoingResponse {
    let response = OutgoingResponse::new(Fields::new());
    response.set_status_code(400).ok();

    // Set Content-Type header for JSON error
    let headers = response.headers();
    headers
        .set(&"content-type".to_string(), &[b"application/json".to_vec()])
        .ok();

    // Write error message to body
    // Per spec: "The HTTP response body MAY comprise a JSON-RPC error response that has no id"
    if let Ok(body) = response.body() {
        if let Ok(stream) = body.write() {
            let error_json = serde_json::json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32700,
                    "message": error
                },
                "id": null
            });
            let error_text = serde_json::to_string(&error_json).unwrap_or_else(|_| error.clone());

            // Write in 4096-byte chunks per WASI spec
            let bytes = error_text.as_bytes();
            let mut offset = 0;
            const MAX_WRITE: usize = 4096;

            while offset < bytes.len() {
                let chunk_size = (bytes.len() - offset).min(MAX_WRITE);
                let chunk = &bytes[offset..offset + chunk_size];
                if stream.blocking_write_and_flush(chunk).is_err() {
                    break;
                }
                offset += chunk_size;
            }

            drop(stream);
            OutgoingBody::finish(body, None).ok();
        }
    }

    response
}

fn create_method_not_allowed_response() -> Result<OutgoingResponse, String> {
    let response = OutgoingResponse::new(Fields::new());
    response
        .set_status_code(405)
        .map_err(|_| "Failed to set status")?;
    Ok(response)
}

bindings::export!(HttpTransport with_types_in bindings);
