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
use crate::bindings::wasmcp::mcp_v20250618::server_handler::{
    ErrorCtx, ResultCtx, Session, handle_error, handle_result,
};
use crate::bindings::wasmcp::mcp_v20250618::session_manager::{
    SessionError, initialize as session_initialize, validate as session_validate,
};
use crate::common;
use crate::config::SessionConfig;

/// Default session store ID for WASI key-value storage
const DEFAULT_SESSION_BUCKET: &str = "";

pub struct HttpTransportGuest;

impl Guest for HttpTransportGuest {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        match handle_http_request(request) {
            Ok(response) => {
                ResponseOutparam::set(response_out, Ok(response));
            }
            Err(e) => {
                // Return error response
                let response = create_error_response(&e);
                ResponseOutparam::set(response_out, Ok(response));
            }
        }
    }
}

fn handle_http_request(request: IncomingRequest) -> Result<OutgoingResponse, String> {
    // 1. Validate Origin header (DNS rebinding protection)
    validate_origin(&request)?;

    // 2. Extract and validate MCP-Protocol-Version header
    let protocol_version = validate_protocol_version(&request)?;

    // 3. Parse method and handle accordingly
    let method = request.method();

    match method {
        Method::Post => handle_post(request, protocol_version),
        Method::Get => handle_get(request, protocol_version),
        Method::Delete => handle_delete(request),
        _ => create_method_not_allowed_response(),
    }
}

fn handle_post(
    request: IncomingRequest,
    protocol_version: String,
) -> Result<OutgoingResponse, String> {
    // Validate Accept header per spec
    validate_accept_header(&request)?;

    // Load session configuration
    let session_config = SessionConfig::from_env();

    // Check for session header and validate if present
    let headers = request.headers();
    let session_id_values = headers.get("mcp-session-id");
    let session_id = if !session_id_values.is_empty() {
        let session_str = String::from_utf8(session_id_values[0].clone())
            .map_err(|_| "HTTP/400:Invalid Mcp-Session-Id header encoding".to_string())?;

        // Only validate if sessions are enabled
        if session_config.enabled {
            let bucket = if session_config.bucket_name.is_empty() {
                DEFAULT_SESSION_BUCKET.to_string()
            } else {
                session_config.bucket_name.clone()
            };

            match session_validate(&session_str, &bucket) {
                Ok(true) => Some(session_str),
                Ok(false) => return Err("HTTP/404:Session terminated".to_string()),
                Err(SessionError::NoSuchSession) => {
                    return Err("HTTP/404:Session not found".to_string());
                }
                Err(_) => return Err("HTTP/500:Session validation error".to_string()),
            }
        } else {
            // Sessions disabled but client sent session ID - ignore it
            None
        }
    } else {
        None
    };

    // Get request body stream
    let body_stream = request.consume().map_err(|_| "Failed to consume request")?;
    let input_stream = body_stream
        .stream()
        .map_err(|_| "Failed to get input stream")?;

    // Parse MCP message
    let message = common::parse_mcp_message(&input_stream)?;

    match message {
        common::McpMessage::Request(request_id, client_request) => {
            // Check if it's initialize - handle specially with plain JSON
            if matches!(client_request, ClientRequest::Initialize(_)) {
                drop(input_stream);
                drop(body_stream);
                return handle_initialize_request(request_id, client_request, protocol_version);
            }

            // Not initialize - check if session is required
            if session_config.enabled && session_id.is_none() {
                drop(input_stream);
                drop(body_stream);
                return Err("HTTP/400:Session ID required for non-initialize requests".to_string());
            }

            // Not initialize - create SSE response for all other requests
            let response = OutgoingResponse::new(Fields::new());
            response
                .set_status_code(200)
                .map_err(|_| "Failed to set status")?;

            let response_headers = response.headers();
            response_headers
                .set("content-type", &[b"text/event-stream".to_vec()])
                .map_err(|_| "Failed to set content-type")?;
            response_headers
                .set("cache-control", &[b"no-cache".to_vec()])
                .map_err(|_| "Failed to set cache-control")?;
            response_headers
                .set("connection", &[b"keep-alive".to_vec()])
                .map_err(|_| "Failed to set connection")?;

            let output_body = response.body().map_err(|_| "Failed to get response body")?;
            let output_stream = output_body
                .write()
                .map_err(|_| "Failed to get output stream")?;

            handle_mcp_request(
                request_id,
                client_request,
                protocol_version,
                session_id.as_deref(),
                &output_stream,
            )?;

            drop(input_stream);
            drop(body_stream);
            drop(output_stream);
            let _ = OutgoingBody::finish(output_body, None);
            Ok(response)
        }
        common::McpMessage::Notification(client_notification) => {
            handle_mcp_notification(client_notification, protocol_version, session_id.as_deref())?;
            drop(input_stream);
            drop(body_stream);
            create_accepted_response()
        }
        common::McpMessage::Result(result_id, client_result) => {
            handle_mcp_result(
                result_id,
                client_result,
                protocol_version,
                session_id.as_deref(),
            )?;
            drop(input_stream);
            drop(body_stream);
            create_accepted_response()
        }
        common::McpMessage::Error(error_id, error_code) => {
            handle_mcp_error(
                error_id,
                error_code,
                protocol_version,
                session_id.as_deref(),
            )?;
            drop(input_stream);
            drop(body_stream);
            create_accepted_response()
        }
    }
}

fn handle_mcp_request(
    request_id: RequestId,
    client_request: ClientRequest,
    protocol_version: String,
    session_id: Option<&str>,
    output_stream: &OutputStream,
) -> Result<(), String> {
    // Parse protocol version
    let proto_ver = parse_protocol_version(&protocol_version)?;

    // Handle based on request type
    match client_request {
        ClientRequest::Initialize(_) => {
            // Should never reach here - initialize is handled separately
            Err("Initialize must be handled before SSE response setup".to_string())
        }
        ClientRequest::Ping(_) => {
            common::handle_ping().map_err(|e| format!("Ping failed: {:?}", e))?;
            common::write_mcp_result(output_stream, &request_id, ServerResult::Ping)
                .map_err(|e| format!("Failed to write ping result: {:?}", e))?;
            Ok(())
        }
        ClientRequest::LoggingSetLevel(level) => {
            let level_str = log_level_to_string(level);
            common::handle_set_log_level(level_str)
                .map_err(|e| format!("SetLevel failed: {:?}", e))?;
            common::write_mcp_result(output_stream, &request_id, ServerResult::LoggingSetLevel)
                .map_err(|e| format!("Failed to write setLevel result: {:?}", e))?;
            Ok(())
        }
        _ => {
            // Load session configuration
            let session_config = SessionConfig::from_env();
            let bucket = if session_config.bucket_name.is_empty() {
                DEFAULT_SESSION_BUCKET.to_string()
            } else {
                session_config.bucket_name.clone()
            };

            // Delegate all other methods to middleware
            let result = common::delegate_to_middleware(
                request_id.clone(),
                client_request,
                proto_ver,
                session_id,
                bucket,
                output_stream,
            )
            .map_err(|e| format!("Middleware delegation failed: {:?}", e))?;

            // Write result via server-io (handles SSE formatting)
            common::write_mcp_result(output_stream, &request_id, result)
                .map_err(|e| format!("Failed to write result: {:?}", e))?;
            Ok(())
        }
    }
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
    let bucket = if session_config.bucket_name.is_empty() {
        DEFAULT_SESSION_BUCKET.to_string()
    } else {
        session_config.bucket_name.clone()
    };

    // Delegate to middleware via notification context
    common::delegate_notification(client_notification, proto_ver, session_id, bucket)
        .map_err(|e| format!("Notification handling failed: {:?}", e))?;

    Ok(())
}

fn handle_get(
    _request: IncomingRequest,
    _protocol_version: String,
) -> Result<OutgoingResponse, String> {
    // GET not supported in stateless mode
    create_method_not_allowed_response()
}

/// Handle DELETE request for session cleanup
fn handle_delete(request: IncomingRequest) -> Result<OutgoingResponse, String> {
    // Load session configuration
    let session_config = SessionConfig::from_env();

    // If sessions not enabled, return 405 Method Not Allowed
    if !session_config.enabled {
        return create_method_not_allowed_response();
    }

    // Extract session ID from header
    let headers = request.headers();
    let session_id_values = headers.get("mcp-session-id");

    let session_id = if !session_id_values.is_empty() {
        String::from_utf8(session_id_values[0].clone())
            .map_err(|_| "HTTP/400:Invalid Mcp-Session-Id header encoding")?
    } else {
        return Err("HTTP/404:No session to delete".to_string());
    };

    // Delete session using session-manager
    let bucket = if session_config.bucket_name.is_empty() {
        DEFAULT_SESSION_BUCKET.to_string()
    } else {
        session_config.bucket_name.clone()
    };

    use crate::bindings::wasmcp::mcp_v20250618::session_manager::delete_session;

    match delete_session(&session_id, &bucket) {
        Ok(_) => {
            // Return 200 OK
            let response = OutgoingResponse::new(Fields::new());
            response
                .set_status_code(200)
                .map_err(|_| "Failed to set status")?;
            Ok(response)
        }
        Err(SessionError::NoSuchSession) => Err("HTTP/404:Session not found".to_string()),
        Err(_) => Err("HTTP/500:Failed to delete session".to_string()),
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
    let response = OutgoingResponse::new(Fields::new());
    response
        .set_status_code(405)
        .map_err(|_| "Failed to set status")?;

    let headers = response.headers();
    headers
        .set("allow", &[b"POST".to_vec()])
        .map_err(|_| "Failed to set allow header")?;

    Ok(response)
}

/// Convert RequestId to JSON value for JSON-RPC responses
fn request_id_to_json(request_id: &RequestId) -> serde_json::Value {
    match request_id {
        RequestId::String(s) => serde_json::Value::String(s.clone()),
        RequestId::Number(n) => serde_json::json!(n),
    }
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
    let session = session_id.map(|id| {
        let store_id = if session_config.bucket_name.is_empty() {
            DEFAULT_SESSION_BUCKET.to_string()
        } else {
            session_config.bucket_name.clone()
        };

        Session {
            session_id: id.to_string(),
            store_id,
        }
    });

    // Create result context
    let ctx = ResultCtx {
        id: result_id,
        protocol_version,
        session,
        user: None,
    };

    // Delegate to imported server-handler (returns unit, not Result)
    handle_result(&ctx, client_result);
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
    let session = session_id.map(|id| {
        let store_id = if session_config.bucket_name.is_empty() {
            DEFAULT_SESSION_BUCKET.to_string()
        } else {
            session_config.bucket_name.clone()
        };

        Session {
            session_id: id.to_string(),
            store_id,
        }
    });

    // Create error context
    let ctx = ErrorCtx {
        id: error_id,
        protocol_version,
        session,
        user: None,
    };

    // Delegate to imported server-handler (returns unit, not Result)
    handle_error(&ctx, &error_code);
    Ok(())
}

/// Handle initialize request - returns plain JSON (not SSE)
fn handle_initialize_request(
    request_id: RequestId,
    _client_request: ClientRequest,
    protocol_version: String,
) -> Result<OutgoingResponse, String> {
    // Parse protocol version
    let proto_ver = parse_protocol_version(&protocol_version)?;

    // Get capabilities from downstream handler
    let capabilities = common::discover_capabilities_for_init(proto_ver);

    // Load session configuration
    let session_config = SessionConfig::from_env();

    // Create session if enabled
    let new_session_id = if session_config.enabled {
        let bucket = if session_config.bucket_name.is_empty() {
            DEFAULT_SESSION_BUCKET.to_string()
        } else {
            session_config.bucket_name.clone()
        };

        match session_initialize(&bucket) {
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
    let headers = Fields::new();
    headers
        .set("content-type", &[b"application/json".to_vec()])
        .map_err(|_| "Failed to set content-type")?;

    // Set Mcp-Session-Id header if session was created
    if let Some(ref session_id) = new_session_id {
        headers
            .set("mcp-session-id", &[session_id.as_bytes().to_vec()])
            .map_err(|_| "Failed to set Mcp-Session-Id header")?;
    }

    let response = OutgoingResponse::new(headers);
    response
        .set_status_code(200)
        .map_err(|_| "Failed to set status")?;

    let body = response.body().map_err(|_| "Failed to get response body")?;
    let output_stream = body.write().map_err(|_| "Failed to get output stream")?;

    // Serialize initialize result
    let result_json = serde_json::json!({
        "jsonrpc": "2.0",
        "id": request_id_to_json(&request_id),
        "result": {
            "protocolVersion": protocol_version_to_string(proto_ver),
            "capabilities": serialize_capabilities(&capabilities),
            "serverInfo": {
                "name": "wasmcp-server",
                "version": env!("CARGO_PKG_VERSION")
            }
        }
    });

    let json_str = serde_json::to_string(&result_json)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

    output_stream
        .blocking_write_and_flush(json_str.as_bytes())
        .map_err(|_| "Failed to write initialize response".to_string())?;

    drop(output_stream);
    let _ = OutgoingBody::finish(body, None);

    Ok(response)
}

/// Serialize ServerCapabilities to JSON
fn serialize_capabilities(
    caps: &crate::bindings::wasmcp::mcp_v20250618::mcp::ServerCapabilities,
) -> serde_json::Value {
    let mut result = serde_json::Map::new();

    if caps.completions.is_some() {
        result.insert("completions".to_string(), serde_json::json!({}));
    }

    if caps.logging.is_some() {
        result.insert("logging".to_string(), serde_json::json!({}));
    }

    if let Some(ref list_changed) = caps.list_changed {
        use crate::bindings::wasmcp::mcp_v20250618::mcp::ServerLists;

        if list_changed.contains(ServerLists::TOOLS) {
            let mut tools_caps = serde_json::Map::new();
            tools_caps.insert("listChanged".to_string(), serde_json::json!(true));
            result.insert("tools".to_string(), serde_json::Value::Object(tools_caps));
        }

        if list_changed.contains(ServerLists::RESOURCES) {
            let mut resources_caps = serde_json::Map::new();
            resources_caps.insert("listChanged".to_string(), serde_json::json!(true));
            result.insert(
                "resources".to_string(),
                serde_json::Value::Object(resources_caps),
            );
        }

        if list_changed.contains(ServerLists::PROMPTS) {
            let mut prompts_caps = serde_json::Map::new();
            prompts_caps.insert("listChanged".to_string(), serde_json::json!(true));
            result.insert(
                "prompts".to_string(),
                serde_json::Value::Object(prompts_caps),
            );
        }
    }

    serde_json::Value::Object(result)
}

/// Create 202 Accepted response
fn create_accepted_response() -> Result<OutgoingResponse, String> {
    let response = OutgoingResponse::new(Fields::new());
    response
        .set_status_code(202)
        .map_err(|_| "Failed to set status")?;
    Ok(response)
}

/// Convert ProtocolVersion to string
fn protocol_version_to_string(version: ProtocolVersion) -> &'static str {
    match version {
        ProtocolVersion::V20250618 => "2025-06-18",
        ProtocolVersion::V20250326 => "2025-03-26",
        ProtocolVersion::V20241105 => "2024-11-05",
    }
}
