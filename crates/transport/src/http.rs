//! HTTP transport implementation
//!
//! Handles HTTP-specific protocol concerns:
//! - Origin validation (DNS rebinding protection)
//! - Header validation (Accept, MCP-Protocol-Version)
//! - HTTP method routing (POST, GET, DELETE)
//! - Request/response lifecycle
//!
//! Delegates I/O to http-server-io via server-io interface

use crate::bindings::exports::wasi::http::incoming_handler::Guest;
use crate::bindings::wasi::cli::environment::get_environment;
use crate::bindings::wasi::http::types::{
    Fields, IncomingRequest, Method, OutgoingBody, OutgoingResponse, ResponseOutparam,
};
use crate::bindings::wasi::io::streams::OutputStream;
use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    ClientNotification, ClientRequest, ClientResult, ErrorCode, LogLevel, ProtocolVersion,
    RequestId, ServerResult,
};
use crate::bindings::wasmcp::mcp_v20250618::server_handler::{MessageContext, Session, handle};
use crate::bindings::wasmcp::mcp_v20250618::session_manager::{
    SessionError, initialize as session_initialize, validate as session_validate,
};
use crate::common;
use crate::config::SessionConfig;

/// Default session store ID for WASI key-value storage
pub(crate) const DEFAULT_SESSION_BUCKET: &str = "";

pub struct HttpTransportGuest;

impl Guest for HttpTransportGuest {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        handle_http_request(request, response_out)
    }
}

fn handle_http_request(request: IncomingRequest, response_out: ResponseOutparam) {
    eprintln!("[TRANSPORT] Handling HTTP request");

    // 1. Validate Origin header (DNS rebinding protection)
    if let Err(e) = validate_origin(&request) {
        let response = create_error_response(&e);
        ResponseOutparam::set(response_out, Ok(response));
        return;
    }
    eprintln!("[TRANSPORT] Origin validated");

    // 2. Extract and validate MCP-Protocol-Version header
    let protocol_version = match validate_protocol_version(&request) {
        Ok(v) => v,
        Err(e) => {
            let response = create_error_response(&e);
            ResponseOutparam::set(response_out, Ok(response));
            return;
        }
    };
    eprintln!("[TRANSPORT] Protocol version: {}", protocol_version);

    // 3. Parse method and handle accordingly
    let method = request.method();
    eprintln!("[TRANSPORT] Method: {:?}", method);

    match method {
        Method::Post => handle_post(request, protocol_version, response_out),
        Method::Get => handle_get(request, protocol_version, response_out),
        Method::Delete => handle_delete(request, response_out),
        _ => match create_method_not_allowed_response() {
            Ok(response) => ResponseOutparam::set(response_out, Ok(response)),
            Err(e) => {
                let response = create_error_response(&e);
                ResponseOutparam::set(response_out, Ok(response));
            }
        },
    }
}

fn handle_post(request: IncomingRequest, protocol_version: String, response_out: ResponseOutparam) {
    eprintln!("[TRANSPORT] Handling POST request");

    // Validate Accept header per spec
    if let Err(e) = validate_accept_header(&request) {
        let response = create_error_response(&e);
        ResponseOutparam::set(response_out, Ok(response));
        return;
    }
    eprintln!("[TRANSPORT] Accept header validated");

    // Load session configuration
    let session_config = SessionConfig::from_env();
    eprintln!(
        "[TRANSPORT] Session config loaded - enabled: {}",
        session_config.enabled
    );

    // Check for session header and validate if present
    let session_id_raw = match extract_session_id_header(&request) {
        Ok(opt) => opt,
        Err(e) => {
            let response = create_error_response(&e);
            ResponseOutparam::set(response_out, Ok(response));
            return;
        }
    };

    let session_id = if let Some(session_str) = session_id_raw {
        // Only validate if sessions are enabled
        if session_config.enabled {
            let bucket = session_config.get_bucket();

            match session_validate(&session_str, bucket) {
                Ok(true) => Some(session_str),
                Ok(false) => {
                    let response = create_error_response("HTTP/404:Session terminated");
                    ResponseOutparam::set(response_out, Ok(response));
                    return;
                }
                Err(SessionError::NoSuchSession) => {
                    let response = create_error_response("HTTP/404:Session not found");
                    ResponseOutparam::set(response_out, Ok(response));
                    return;
                }
                Err(_) => {
                    let response = create_error_response("HTTP/500:Session validation error");
                    ResponseOutparam::set(response_out, Ok(response));
                    return;
                }
            }
        } else {
            // Sessions disabled but client sent session ID - ignore it
            None
        }
    } else {
        None
    };

    // Get request body stream
    let body_stream = match request.consume() {
        Ok(s) => s,
        Err(_) => {
            let response = create_error_response("Failed to consume request");
            ResponseOutparam::set(response_out, Ok(response));
            return;
        }
    };
    let input_stream = match body_stream.stream() {
        Ok(s) => s,
        Err(_) => {
            let response = create_error_response("Failed to get input stream");
            ResponseOutparam::set(response_out, Ok(response));
            return;
        }
    };

    // Parse MCP message
    let message = match common::parse_mcp_message(
        &input_stream,
        common::http_read_limit(),
        &common::plain_json_frame(),
    ) {
        Ok(m) => m,
        Err(e) => {
            let response = create_error_response(&e);
            ResponseOutparam::set(response_out, Ok(response));
            return;
        }
    };

    match message {
        common::McpMessage::Request(request_id, client_request) => {
            // Check if it's initialize - handle specially with plain JSON
            if matches!(client_request, ClientRequest::Initialize(_)) {
                drop(input_stream);
                drop(body_stream);
                handle_initialize_request(
                    request_id,
                    client_request,
                    protocol_version,
                    response_out,
                );
                return;
            }

            // Not initialize - check if session is required
            if session_config.enabled && session_id.is_none() {
                drop(input_stream);
                drop(body_stream);
                let response = create_error_response(
                    "HTTP/400:Session ID required for non-initialize requests",
                );
                ResponseOutparam::set(response_out, Ok(response));
                return;
            }

            // Not initialize - create SSE response for all other requests
            eprintln!("[TRANSPORT] Creating SSE response for non-initialize request");
            let response_headers = Fields::new();
            if let Err(_) = response_headers.set("content-type", &[b"text/event-stream".to_vec()]) {
                let response = create_error_response("Failed to set content-type");
                ResponseOutparam::set(response_out, Ok(response));
                return;
            }
            if let Err(_) = response_headers.set("cache-control", &[b"no-cache".to_vec()]) {
                let response = create_error_response("Failed to set cache-control");
                ResponseOutparam::set(response_out, Ok(response));
                return;
            }
            eprintln!(
                "[TRANSPORT] SSE headers set (content-type: text/event-stream, cache-control: no-cache)"
            );

            let response = OutgoingResponse::new(response_headers);
            if let Err(_) = response.set_status_code(200) {
                let err_response = create_error_response("Failed to set status");
                ResponseOutparam::set(response_out, Ok(err_response));
                return;
            }
            eprintln!("[TRANSPORT] SSE Response created with status 200");

            let output_body = match response.body() {
                Ok(b) => b,
                Err(_) => {
                    let err_response = create_error_response("Failed to get response body");
                    ResponseOutparam::set(response_out, Ok(err_response));
                    return;
                }
            };
            eprintln!("[TRANSPORT] Response body obtained");

            let output_stream = match output_body.write() {
                Ok(s) => s,
                Err(_) => {
                    let err_response = create_error_response("Failed to get output stream");
                    ResponseOutparam::set(response_out, Ok(err_response));
                    return;
                }
            };
            eprintln!("[TRANSPORT] Output stream acquired for SSE streaming");

            // *** CRITICAL: Set response BEFORE processing request ***
            // This allows client to start reading while we write notifications
            eprintln!("[TRANSPORT] Setting response (enables true SSE streaming)");
            ResponseOutparam::set(response_out, Ok(response));
            eprintln!("[TRANSPORT] Response set - client can now read stream concurrently");

            eprintln!("[TRANSPORT] Processing MCP request (streaming notifications via SSE)...");
            if let Err(e) = handle_mcp_request(
                request_id,
                client_request,
                protocol_version,
                session_id.as_deref(),
                &output_stream,
                &common::http_sse_frame(),
            ) {
                eprintln!("[TRANSPORT] ERROR during request processing: {}", e);
                // Can't send error response now - already set response
                // Just finish the body
            }
            eprintln!("[TRANSPORT] MCP request processing complete, all notifications sent");

            eprintln!("[TRANSPORT] Dropping streams...");
            drop(input_stream);
            drop(body_stream);
            drop(output_stream);
            eprintln!("[TRANSPORT] Streams dropped, finishing body...");
            if let Err(e) = OutgoingBody::finish(output_body, None) {
                eprintln!("[TRANSPORT] ERROR finishing body: {:?}", e);
            }
            eprintln!("[TRANSPORT] OutgoingBody::finish() completed - SSE response complete");
        }
        common::McpMessage::Notification(client_notification) => {
            let result = handle_mcp_notification(
                client_notification,
                protocol_version,
                session_id.as_deref(),
            );
            drop(input_stream);
            drop(body_stream);
            respond_with_result(result, response_out);
        }
        common::McpMessage::Result(result_id, client_result) => {
            let result = handle_mcp_result(
                result_id,
                client_result,
                protocol_version,
                session_id.as_deref(),
            );
            drop(input_stream);
            drop(body_stream);
            respond_with_result(result, response_out);
        }
        common::McpMessage::Error(error_id, error_code) => {
            let result = handle_mcp_error(
                error_id,
                error_code,
                protocol_version,
                session_id.as_deref(),
            );
            drop(input_stream);
            drop(body_stream);
            respond_with_result(result, response_out);
        }
    }
}

fn handle_mcp_request(
    request_id: RequestId,
    client_request: ClientRequest,
    protocol_version: String,
    session_id: Option<&str>,
    output_stream: &OutputStream,
    frame: &common::MessageFrame,
) -> Result<(), String> {
    eprintln!("[TRANSPORT-MCP] Starting request handler");
    // Parse protocol version
    let proto_ver = parse_protocol_version(&protocol_version)?;
    eprintln!("[TRANSPORT-MCP] Protocol version: {:?}", proto_ver);

    // Handle based on request type
    match client_request {
        ClientRequest::Initialize(_) => {
            eprintln!("[TRANSPORT-MCP] ERROR: Initialize should never reach here");
            // Should never reach here - initialize is handled separately
            Err("Initialize must be handled before SSE response setup".to_string())
        }
        ClientRequest::Ping(_) => {
            eprintln!("[TRANSPORT-MCP] Handling Ping");
            common::handle_ping().map_err(|e| format!("Ping failed: {:?}", e))?;
            common::write_mcp_result(output_stream, request_id, ServerResult::Ping, frame)
                .map_err(|e| format!("Failed to write ping result: {:?}", e))?;
            eprintln!("[TRANSPORT-MCP] Ping response sent");
            Ok(())
        }
        ClientRequest::LoggingSetLevel(level) => {
            eprintln!("[TRANSPORT-MCP] Handling LoggingSetLevel: {:?}", level);
            let level_str = log_level_to_string(level);
            common::handle_set_log_level(level_str)
                .map_err(|e| format!("SetLevel failed: {:?}", e))?;
            common::write_mcp_result(
                output_stream,
                request_id,
                ServerResult::LoggingSetLevel,
                frame,
            )
            .map_err(|e| format!("Failed to write setLevel result: {:?}", e))?;
            eprintln!("[TRANSPORT-MCP] LoggingSetLevel response sent");
            Ok(())
        }
        _ => {
            eprintln!(
                "[TRANSPORT-MCP] Delegating to middleware for request type: {:?}",
                std::any::type_name_of_val(&client_request)
            );
            // Load session configuration
            let session_config = SessionConfig::from_env();
            let bucket = session_config.get_bucket().to_string();

            // Delegate all other methods to middleware
            eprintln!(
                "[TRANSPORT-MCP] Calling delegate_to_middleware (notifications will stream via SSE)..."
            );
            let result = common::delegate_to_middleware(
                request_id.clone(),
                client_request,
                proto_ver,
                session_id,
                bucket,
                output_stream,
                frame,
            )
            .map_err(|e| format!("Middleware delegation failed: {:?}", e))?;

            eprintln!("[TRANSPORT-MCP] Middleware returned final result, writing to SSE stream...");
            // Write result via server-io (handles SSE formatting)
            common::write_mcp_result(output_stream, request_id, result, frame)
                .map_err(|e| format!("Failed to write result: {:?}", e))?;
            eprintln!("[TRANSPORT-MCP] Final result written to SSE stream");
            Ok(())
        }
    }
}

/// Create session from session ID and config
fn create_session(session_id: Option<&str>, config: &SessionConfig) -> Option<Session> {
    session_id.map(|id| Session {
        session_id: id.to_string(),
        store_id: config.get_bucket().to_string(),
    })
}

/// Send result or error response for notifications, results, and errors
fn respond_with_result(result: Result<(), String>, response_out: ResponseOutparam) {
    match result {
        Ok(_) => match create_accepted_response() {
            Ok(response) => ResponseOutparam::set(response_out, Ok(response)),
            Err(e) => {
                let response = create_error_response(&e);
                ResponseOutparam::set(response_out, Ok(response));
            }
        },
        Err(e) => {
            let response = create_error_response(&e);
            ResponseOutparam::set(response_out, Ok(response));
        }
    }
}

/// Extract session ID from request headers
fn extract_session_id_header(request: &IncomingRequest) -> Result<Option<String>, String> {
    let headers = request.headers();
    let session_id_values = headers.get("mcp-session-id");

    if session_id_values.is_empty() {
        return Ok(None);
    }

    String::from_utf8(session_id_values[0].clone())
        .map(Some)
        .map_err(|_| "HTTP/400:Invalid Mcp-Session-Id header encoding".to_string())
}

fn handle_mcp_notification(
    client_notification: ClientNotification,
    protocol_version: String,
    session_id: Option<&str>,
) -> Result<(), String> {
    // Parse protocol version
    let proto_ver = parse_protocol_version(&protocol_version)?;

    // Load session configuration
    let session_config = SessionConfig::from_env();
    let bucket = session_config.get_bucket().to_string();

    // Delegate to middleware via notification context
    common::delegate_notification(
        client_notification,
        proto_ver,
        session_id,
        bucket,
        &common::plain_json_frame(),
    )
    .map_err(|e| format!("Notification handling failed: {:?}", e))?;

    Ok(())
}

fn handle_get(
    _request: IncomingRequest,
    _protocol_version: String,
    response_out: ResponseOutparam,
) {
    eprintln!("[TRANSPORT] GET request received - returning 405 Method Not Allowed");
    match create_method_not_allowed_response() {
        Ok(response) => {
            eprintln!("[TRANSPORT] 405 response created successfully");
            ResponseOutparam::set(response_out, Ok(response));
        }
        Err(e) => {
            eprintln!("[TRANSPORT] Error creating 405 response: {}", e);
            let response = create_error_response(&e);
            ResponseOutparam::set(response_out, Ok(response));
        }
    }
}

/// Handle DELETE request for session cleanup
fn handle_delete(request: IncomingRequest, response_out: ResponseOutparam) {
    // Load session configuration
    let session_config = SessionConfig::from_env();

    // If sessions not enabled, return 405 Method Not Allowed
    if !session_config.enabled {
        match create_method_not_allowed_response() {
            Ok(response) => ResponseOutparam::set(response_out, Ok(response)),
            Err(e) => {
                let response = create_error_response(&e);
                ResponseOutparam::set(response_out, Ok(response));
            }
        }
        return;
    }

    // Extract session ID from header
    let session_id = match extract_session_id_header(&request) {
        Ok(Some(id)) => id,
        Ok(None) => {
            let response = create_error_response("HTTP/404:No session to delete");
            ResponseOutparam::set(response_out, Ok(response));
            return;
        }
        Err(e) => {
            let response = create_error_response(&e);
            ResponseOutparam::set(response_out, Ok(response));
            return;
        }
    };

    // Delete session using session-manager
    let bucket = session_config.get_bucket().to_string();

    use crate::bindings::wasmcp::mcp_v20250618::session_manager::delete_session;

    match delete_session(&session_id, &bucket) {
        Ok(_) => {
            // Return 200 OK
            let response = OutgoingResponse::new(Fields::new());
            if let Err(_) = response.set_status_code(200) {
                let err_response = create_error_response("Failed to set status");
                ResponseOutparam::set(response_out, Ok(err_response));
                return;
            }
            ResponseOutparam::set(response_out, Ok(response));
        }
        Err(SessionError::NoSuchSession) => {
            let response = create_error_response("HTTP/404:Session not found");
            ResponseOutparam::set(response_out, Ok(response));
        }
        Err(_) => {
            let response = create_error_response("HTTP/500:Failed to delete session");
            ResponseOutparam::set(response_out, Ok(response));
        }
    }
}

/// Validate Accept header per MCP spec
fn validate_accept_header(request: &IncomingRequest) -> Result<(), String> {
    let headers = request.headers();
    let accept_values = headers.get("accept");

    if accept_values.is_empty() {
        return Err("Missing Accept header".to_string());
    }

    let accept_str = String::from_utf8(accept_values[0].clone())
        .map_err(|_| "Invalid Accept header encoding".to_string())?;

    let has_json = accept_str.contains("application/json") || accept_str.contains("*/*");
    let has_sse = accept_str.contains("text/event-stream") || accept_str.contains("*/*");

    if !has_json || !has_sse {
        return Err(
            "Accept header must include both application/json and text/event-stream".to_string(),
        );
    }

    Ok(())
}

/// Validate MCP-Protocol-Version header
fn validate_protocol_version(request: &IncomingRequest) -> Result<String, String> {
    let headers = request.headers();
    let version_values = headers.get("mcp-protocol-version");

    if version_values.is_empty() {
        // Default to 2025-03-26 for backwards compatibility
        return Ok("2025-03-26".to_string());
    }

    let version_str = String::from_utf8(version_values[0].clone())
        .map_err(|_| "Invalid MCP-Protocol-Version header encoding".to_string())?;

    match version_str.as_str() {
        "2025-06-18" | "2025-03-26" | "2024-11-05" => Ok(version_str),
        _ => Err(format!("Unsupported MCP-Protocol-Version: {}", version_str)),
    }
}

/// Validate Origin header to prevent DNS rebinding attacks
fn validate_origin(request: &IncomingRequest) -> Result<(), String> {
    let headers = request.headers();
    let origin_values = headers.get("origin");

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

    let origin = if origin_values.is_empty() {
        if require_origin == "true" {
            return Err("Origin header required but not provided".to_string());
        }
        return Ok(());
    } else {
        String::from_utf8(origin_values[0].clone())
            .map_err(|_| "Invalid Origin header encoding".to_string())?
    };

    match allowed_origins {
        Some(allowed) => {
            let allowed_list: Vec<&str> = allowed.split(',').map(|s| s.trim()).collect();

            if allowed_list.contains(&"*") {
                return Ok(());
            }

            if allowed_list.contains(&origin.as_str()) {
                Ok(())
            } else {
                Err(format!(
                    "Origin '{}' not in allowed list. Set MCP_ALLOWED_ORIGINS environment variable.",
                    origin
                ))
            }
        }
        None => validate_localhost_origin(&origin),
    }
}

/// Validate localhost origin (default secure behavior)
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
        "Origin '{}' not allowed. By default, only localhost origins are permitted.",
        origin
    ))
}

/// Create error response
fn create_error_response(message: &str) -> OutgoingResponse {
    // Parse HTTP status code from message prefix (format: "HTTP/404:message")
    let (status_code, error_message) = if message.starts_with("HTTP/") {
        let parts: Vec<&str> = message.splitn(2, ':').collect();
        if parts.len() == 2 {
            let status_str = parts[0].strip_prefix("HTTP/").unwrap_or("400");
            let status = status_str.parse::<u16>().unwrap_or(400);
            (status, parts[1])
        } else {
            (400, message)
        }
    } else {
        (400, message)
    };

    let response = OutgoingResponse::new(Fields::new());
    let _ = response.set_status_code(status_code);

    let headers = response.headers();
    let _ = headers.set("content-type", &[b"application/json".to_vec()]);

    if let Ok(body) = response.body() {
        if let Ok(stream) = body.write() {
            let error_json = serde_json::json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": {
                    "code": -32700,
                    "message": error_message
                }
            });
            let _ = stream.blocking_write_and_flush(error_json.to_string().as_bytes());
            drop(stream);
        }
        let _ = OutgoingBody::finish(body, None);
    }

    response
}

/// Create 405 Method Not Allowed response
fn create_method_not_allowed_response() -> Result<OutgoingResponse, String> {
    // Set Allow header based on session support
    let session_config = SessionConfig::from_env();
    let allow_methods = if session_config.enabled {
        b"POST, DELETE".to_vec()
    } else {
        b"POST".to_vec()
    };

    // Create headers first
    let headers = Fields::new();
    headers
        .set("allow", &[allow_methods])
        .map_err(|_| "Failed to set allow header")?;

    // Create response with headers
    let response = OutgoingResponse::new(headers);
    response
        .set_status_code(405)
        .map_err(|_| "Failed to set status")?;

    Ok(response)
}

/// Parse protocol version string to enum
fn parse_protocol_version(version: &str) -> Result<ProtocolVersion, String> {
    match version {
        "2025-06-18" => Ok(ProtocolVersion::V20250618),
        "2025-03-26" => Ok(ProtocolVersion::V20250326),
        "2024-11-05" => Ok(ProtocolVersion::V20241105),
        _ => Err(format!("Unsupported protocol version: {}", version)),
    }
}

/// Convert LogLevel enum to string
fn log_level_to_string(level: LogLevel) -> String {
    match level {
        LogLevel::Debug => "debug".to_string(),
        LogLevel::Info => "info".to_string(),
        LogLevel::Notice => "notice".to_string(),
        LogLevel::Warning => "warning".to_string(),
        LogLevel::Error => "error".to_string(),
        LogLevel::Critical => "critical".to_string(),
        LogLevel::Alert => "alert".to_string(),
        LogLevel::Emergency => "emergency".to_string(),
    }
}

/// Handle result response from client (bidirectional MCP)
fn handle_mcp_result(
    result_id: RequestId,
    client_result: ClientResult,
    protocol_version: String,
    session_id: Option<&str>,
) -> Result<(), String> {
    // Load session configuration
    let session_config = SessionConfig::from_env();

    // Create session if provided
    let session = create_session(session_id, &session_config);

    // Create message context (no client-stream for results - client sending to server)
    let ctx = MessageContext {
        client_stream: None,
        protocol_version,
        session,
        identity: None,
        frame: common::plain_json_frame(),
    };

    // Create client message
    let message = crate::bindings::wasmcp::mcp_v20250618::mcp::ClientMessage::Result((
        result_id,
        client_result,
    ));

    // Delegate to imported server-handler (should return None for results from client)
    handle(&ctx, message);
    Ok(())
}

/// Handle error response from client (bidirectional MCP)
fn handle_mcp_error(
    error_id: Option<RequestId>,
    error_code: ErrorCode,
    protocol_version: String,
    session_id: Option<&str>,
) -> Result<(), String> {
    // Load session configuration
    let session_config = SessionConfig::from_env();

    // Create session if provided
    let session = create_session(session_id, &session_config);

    // Create message context (no client-stream for errors - client sending to server)
    let ctx = MessageContext {
        client_stream: None,
        protocol_version,
        session,
        identity: None,
        frame: common::plain_json_frame(),
    };

    // Create client message
    let message =
        crate::bindings::wasmcp::mcp_v20250618::mcp::ClientMessage::Error((error_id, error_code));

    // Delegate to imported server-handler (should return None for errors from client)
    handle(&ctx, message);
    Ok(())
}

/// Handle initialize request - returns plain JSON (not SSE)
fn handle_initialize_request(
    request_id: RequestId,
    _client_request: ClientRequest,
    protocol_version: String,
    response_out: ResponseOutparam,
) {
    eprintln!("[TRANSPORT] Handling initialize request");

    // Parse protocol version
    let proto_ver = match parse_protocol_version(&protocol_version) {
        Ok(v) => v,
        Err(e) => {
            let response = create_error_response(&e);
            ResponseOutparam::set(response_out, Ok(response));
            return;
        }
    };
    eprintln!("[TRANSPORT] Protocol version parsed: {:?}", proto_ver);

    // Get capabilities from downstream handler
    let capabilities =
        common::discover_capabilities_for_init(proto_ver, &common::plain_json_frame());
    eprintln!("[TRANSPORT] Capabilities discovered");

    // Load session configuration
    let session_config = SessionConfig::from_env();
    eprintln!("[TRANSPORT] Session config loaded for init");

    // Create session if enabled
    let new_session_id = if session_config.enabled {
        let bucket = session_config.get_bucket();

        match session_initialize(bucket) {
            Ok(id) => Some(id),
            Err(_) => {
                // Session creation failed - continue without session (non-fatal per MCP spec)
                eprintln!("[WARN] Session creation failed, continuing without session support");
                None
            }
        }
    } else {
        None
    };

    // Create plain JSON response with optional session header
    eprintln!("[TRANSPORT] Creating response headers for initialize");
    let headers = Fields::new();
    eprintln!("[TRANSPORT] Setting content-type header");
    if let Err(_) = headers.set("content-type", &[b"application/json".to_vec()]) {
        let response = create_error_response("Failed to set content-type");
        ResponseOutparam::set(response_out, Ok(response));
        return;
    }
    eprintln!("[TRANSPORT] Content-type header set successfully");

    // Set Mcp-Session-Id header if session was created
    if let Some(ref session_id) = new_session_id {
        eprintln!("[TRANSPORT] Setting Mcp-Session-Id header: {}", session_id);
        if let Err(_) = headers.set("mcp-session-id", &[session_id.as_bytes().to_vec()]) {
            let response = create_error_response("Failed to set Mcp-Session-Id header");
            ResponseOutparam::set(response_out, Ok(response));
            return;
        }
        eprintln!("[TRANSPORT] Mcp-Session-Id header set successfully");
    }

    eprintln!("[TRANSPORT] Creating OutgoingResponse");
    let response = OutgoingResponse::new(headers);
    eprintln!("[TRANSPORT] Setting status code 200");
    if let Err(_) = response.set_status_code(200) {
        let err_response = create_error_response("Failed to set status");
        ResponseOutparam::set(response_out, Ok(err_response));
        return;
    }
    eprintln!("[TRANSPORT] Status code set successfully");

    eprintln!("[TRANSPORT] Getting response body");
    let body = match response.body() {
        Ok(b) => b,
        Err(_) => {
            let err_response = create_error_response("Failed to get response body");
            ResponseOutparam::set(response_out, Ok(err_response));
            return;
        }
    };
    eprintln!("[TRANSPORT] Getting output stream");
    let output_stream = match body.write() {
        Ok(s) => s,
        Err(_) => {
            let err_response = create_error_response("Failed to get output stream");
            ResponseOutparam::set(response_out, Ok(err_response));
            return;
        }
    };
    eprintln!("[TRANSPORT] Output stream acquired");

    // Build InitializeResult using MCP types
    use crate::bindings::wasmcp::mcp_v20250618::mcp::{Implementation, InitializeResult};

    let init_result = InitializeResult {
        meta: None,
        server_info: Implementation {
            name: "wasmcp-server".to_string(),
            title: Some("wasmcp Universal Transport Server".to_string()),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        capabilities,
        protocol_version: proto_ver,
        options: None,
    };

    // Construct ServerMessage
    use crate::bindings::wasmcp::mcp_v20250618::mcp::{ServerMessage, ServerResult};
    let server_message = ServerMessage::Result((request_id, ServerResult::Initialize(init_result)));

    eprintln!("[TRANSPORT] Sending initialize response via server-io");
    // Use server-io to write the message with plain JSON framing
    use crate::bindings::wasmcp::mcp_v20250618::server_io;
    if let Err(e) = server_io::send_message(
        &output_stream,
        server_message,
        &crate::common::plain_json_frame(),
    ) {
        eprintln!("[TRANSPORT] ERROR sending initialize response: {:?}", e);
        let err_response =
            create_error_response(&format!("Failed to send initialize response: {:?}", e));
        ResponseOutparam::set(response_out, Ok(err_response));
        return;
    }
    eprintln!("[TRANSPORT] Initialize response sent successfully");

    drop(output_stream);
    if let Err(e) = OutgoingBody::finish(body, None) {
        eprintln!("[TRANSPORT] ERROR finishing body: {:?}", e);
        let err_response = create_error_response("Failed to finish body");
        ResponseOutparam::set(response_out, Ok(err_response));
        return;
    }

    ResponseOutparam::set(response_out, Ok(response));
}

/// Create 202 Accepted response
fn create_accepted_response() -> Result<OutgoingResponse, String> {
    let response = OutgoingResponse::new(Fields::new());
    response
        .set_status_code(202)
        .map_err(|_| "Failed to set status")?;
    Ok(response)
}
