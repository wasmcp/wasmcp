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
    ClientNotification, ClientRequest, ClientResult, ErrorCode, RequestId, ServerResult,
};
use crate::bindings::wasmcp::mcp_v20250618::server_handler::handle;
use crate::bindings::wasmcp::mcp_v20250618::session_manager::{
    SessionError, initialize as session_initialize, validate as session_validate,
};
use crate::common;
use crate::config::SessionConfig;
use crate::error::TransportError;

/// Default session store ID for WASI key-value storage
pub(crate) const DEFAULT_SESSION_BUCKET: &str = "";

pub struct HttpTransportGuest;

impl Guest for HttpTransportGuest {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        // Block on async handler - this bridges sync WIT trait to async implementation
        futures::executor::block_on(handle_http_request_async(request, response_out))
    }
}

async fn handle_http_request_async(request: IncomingRequest, response_out: ResponseOutparam) {
    eprintln!("[TRANSPORT] Handling HTTP request");

    // 1. Load session configuration once for the entire request
    let session_config = SessionConfig::from_env();
    eprintln!(
        "[TRANSPORT] Session config loaded - enabled: {}, mode: {:?}",
        session_config.enabled, session_config.mode
    );

    // 2. Validate Origin header (DNS rebinding protection)
    if let Err(e) = validate_origin(&request) {
        let response = transport_error_to_response(&e);
        ResponseOutparam::set(response_out, Ok(response));
        return;
    }
    eprintln!("[TRANSPORT] Origin validated");

    // 3. Extract and validate MCP-Protocol-Version header
    let protocol_version = match validate_protocol_version(&request) {
        Ok(v) => v,
        Err(e) => {
            let response = transport_error_to_response(&e);
            ResponseOutparam::set(response_out, Ok(response));
            return;
        }
    };
    eprintln!("[TRANSPORT] Protocol version: {}", protocol_version);

    // 4. Parse method and handle accordingly
    let method = request.method();
    eprintln!("[TRANSPORT] Method: {:?}", method);

    match method {
        Method::Post => handle_post(request, protocol_version, response_out, &session_config).await,
        Method::Get => handle_get(request, protocol_version, response_out, &session_config),
        Method::Delete => handle_delete(request, response_out, &session_config),
        _ => match create_method_not_allowed_response(&session_config) {
            Ok(response) => ResponseOutparam::set(response_out, Ok(response)),
            Err(e) => {
                let error = TransportError::internal(e);
                let response = transport_error_to_response(&error);
                ResponseOutparam::set(response_out, Ok(response));
            }
        },
    }
}

async fn handle_post(
    request: IncomingRequest,
    protocol_version: String,
    response_out: ResponseOutparam,
    session_config: &SessionConfig,
) {
    eprintln!("[TRANSPORT] Handling POST request");

    // Validate Accept header per spec
    if let Err(e) = validate_accept_header(&request) {
        let response = transport_error_to_response(&e);
        ResponseOutparam::set(response_out, Ok(response));
        return;
    }
    eprintln!("[TRANSPORT] Accept header validated");

    // Check for session header and validate if present
    let session_id_raw = match extract_session_id_header(&request) {
        Ok(opt) => opt,
        Err(e) => {
            let response = transport_error_to_response(&e);
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
                    let error = TransportError::session("Session terminated");
                    let response = transport_error_to_response(&error);
                    ResponseOutparam::set(response_out, Ok(response));
                    return;
                }
                Err(SessionError::NoSuchSession) => {
                    let error = TransportError::session("Session not found");
                    let response = transport_error_to_response(&error);
                    ResponseOutparam::set(response_out, Ok(response));
                    return;
                }
                Err(_) => {
                    let error = TransportError::session("Session validation error");
                    let response = transport_error_to_response(&error);
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
            let error = TransportError::internal("Failed to consume request");
            let response = transport_error_to_response(&error);
            ResponseOutparam::set(response_out, Ok(response));
            return;
        }
    };
    let input_stream = match body_stream.stream() {
        Ok(s) => s,
        Err(_) => {
            let error = TransportError::internal("Failed to get input stream");
            let response = transport_error_to_response(&error);
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
            let error = TransportError::protocol(e);
            let response = transport_error_to_response(&error);
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
                    session_config,
                );
                return;
            }

            // Not initialize - check if session is required
            if session_config.enabled && session_id.is_none() {
                drop(input_stream);
                drop(body_stream);
                let error =
                    TransportError::session("Session ID required for non-initialize requests");
                let response = transport_error_to_response(&error);
                ResponseOutparam::set(response_out, Ok(response));
                return;
            }

            // Not initialize - delegate to mode-specific handler
            use crate::config::ServerMode;

            match session_config.mode {
                ServerMode::Json => handle_json_mode(
                    request_id,
                    client_request,
                    protocol_version,
                    session_id.as_deref(),
                    input_stream,
                    body_stream,
                    response_out,
                    session_config,
                ),
                ServerMode::Sse => {
                    handle_sse_streaming_mode(
                        request_id,
                        client_request,
                        protocol_version,
                        session_id.as_deref(),
                        input_stream,
                        body_stream,
                        response_out,
                        session_config,
                    )
                    .await
                }
                ServerMode::SseBuffer => handle_sse_buffered_mode(
                    request_id,
                    client_request,
                    protocol_version,
                    session_id.as_deref(),
                    input_stream,
                    body_stream,
                    response_out,
                    session_config,
                ),
            }
        }
        common::McpMessage::Notification(client_notification) => {
            let result = handle_mcp_notification(
                client_notification,
                protocol_version,
                session_id.as_deref(),
                session_config,
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
                session_config,
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
                session_config,
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
    config: &SessionConfig,
) -> Result<(), TransportError> {
    eprintln!("[TRANSPORT-MCP] Starting request handler");
    // Parse protocol version
    let proto_ver =
        common::parse_protocol_version(&protocol_version).map_err(TransportError::protocol)?;
    eprintln!("[TRANSPORT-MCP] Protocol version: {:?}", proto_ver);

    // Handle based on request type
    match client_request {
        ClientRequest::Initialize(_) => {
            eprintln!("[TRANSPORT-MCP] ERROR: Initialize should never reach here");
            // Should never reach here - initialize is handled separately
            Err(TransportError::internal(
                "Initialize must be handled before SSE response setup",
            ))
        }
        ClientRequest::Ping(_) => {
            eprintln!("[TRANSPORT-MCP] Handling Ping");
            common::handle_ping()
                .map_err(|e| TransportError::protocol(format!("Ping failed: {:?}", e)))?;
            common::write_mcp_result(output_stream, request_id, ServerResult::Ping, frame)?;
            eprintln!("[TRANSPORT-MCP] Ping response sent");
            Ok(())
        }
        ClientRequest::LoggingSetLevel(level) => {
            eprintln!("[TRANSPORT-MCP] Handling LoggingSetLevel: {:?}", level);
            let level_str = common::log_level_to_string(level);
            common::handle_set_log_level(level_str)
                .map_err(|e| TransportError::protocol(format!("SetLevel failed: {:?}", e)))?;
            common::write_mcp_result(
                output_stream,
                request_id,
                ServerResult::LoggingSetLevel,
                frame,
            )?;
            eprintln!("[TRANSPORT-MCP] LoggingSetLevel response sent");
            Ok(())
        }
        _ => {
            eprintln!(
                "[TRANSPORT-MCP] Delegating to middleware for request type: {:?}",
                std::any::type_name_of_val(&client_request)
            );
            let bucket = config.get_bucket().to_string();

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
            .map_err(|e| {
                TransportError::protocol(format!("Middleware delegation failed: {:?}", e))
            })?;

            eprintln!("[TRANSPORT-MCP] Middleware returned final result, writing to SSE stream...");
            // Write result via server-io (handles SSE formatting)
            common::write_mcp_result(output_stream, request_id, result, frame)?;
            eprintln!("[TRANSPORT-MCP] Final result written to SSE stream");
            Ok(())
        }
    }
}

/// Handle POST request in JSON mode (single buffered response)
fn handle_json_mode(
    request_id: RequestId,
    client_request: ClientRequest,
    protocol_version: String,
    session_id: Option<&str>,
    input_stream: crate::bindings::wasi::io::streams::InputStream,
    body_stream: crate::bindings::wasi::http::types::IncomingBody,
    response_out: ResponseOutparam,
    config: &SessionConfig,
) {
    use crate::bindings::wasi::http::types::Fields;
    use crate::bindings::wasi::http::types::{OutgoingBody, OutgoingResponse};
    use crate::bindings::wasmcp::mcp_v20250618::server_io;

    eprintln!("[TRANSPORT] Using JSON mode (single buffered response)");

    let response_headers = Fields::new();
    if response_headers
        .set("content-type", &[b"application/json".to_vec()])
        .is_err()
    {
        let error = TransportError::internal("Failed to set content-type");
        let response = transport_error_to_response(&error);
        ResponseOutparam::set(response_out, Ok(response));
        return;
    }

    let response = OutgoingResponse::new(response_headers);
    if response.set_status_code(200).is_err() {
        let error = TransportError::internal("Failed to set status");
        let err_response = transport_error_to_response(&error);
        ResponseOutparam::set(response_out, Ok(err_response));
        return;
    }

    let output_body = match response.body() {
        Ok(b) => b,
        Err(_) => {
            let error = TransportError::internal("Failed to get response body");
            let err_response = transport_error_to_response(&error);
            ResponseOutparam::set(response_out, Ok(err_response));
            return;
        }
    };

    let output_stream = match output_body.write() {
        Ok(s) => s,
        Err(_) => {
            let error = TransportError::internal("Failed to get output stream");
            let err_response = transport_error_to_response(&error);
            ResponseOutparam::set(response_out, Ok(err_response));
            return;
        }
    };

    // Process request with plain JSON framing
    if let Err(e) = handle_mcp_request(
        request_id.clone(),
        client_request,
        protocol_version,
        session_id,
        &output_stream,
        &common::plain_json_frame(),
        config,
    ) {
        eprintln!("[TRANSPORT] ERROR during request processing: {:?}", e);
        // Write error response to stream
        use crate::bindings::wasmcp::mcp_v20250618::mcp::{Error, ErrorCode, ServerMessage};
        let error_code = ErrorCode::InternalError(Error {
            code: -32603,
            message: e.message(),
            data: None,
        });
        let error_message = ServerMessage::Error((Some(request_id), error_code));
        let _ = crate::bindings::wasmcp::mcp_v20250618::server_io::send_message(
            &output_stream,
            error_message,
            &common::plain_json_frame(),
        );
    }

    // Flush buffer (single write)
    if let Err(e) = server_io::flush_buffer(&output_stream) {
        eprintln!("[TRANSPORT] ERROR flushing buffer: {:?}", e);
    }

    // Clean up - drop streams BEFORE finishing body (output_stream is child of output_body)
    drop(output_stream);
    drop(input_stream);
    drop(body_stream);
    if let Err(e) = OutgoingBody::finish(output_body, None) {
        eprintln!("[TRANSPORT] ERROR finishing body: {:?}", e);
    }

    // Set response AFTER all writes complete
    ResponseOutparam::set(response_out, Ok(response));
    eprintln!("[TRANSPORT] JSON response complete");
}

/// Handle POST request in SSE streaming mode (async writes with yielding)
///
/// Mimics Spin SDK's async streaming pattern to avoid stream budget exhaustion:
/// - Writes with check_write() to respect backpressure
/// - Yields to async executor between writes via .await
/// - Allows channel buffer to drain between messages
async fn handle_sse_streaming_mode(
    request_id: RequestId,
    client_request: ClientRequest,
    protocol_version: String,
    session_id: Option<&str>,
    input_stream: crate::bindings::wasi::io::streams::InputStream,
    body_stream: crate::bindings::wasi::http::types::IncomingBody,
    response_out: ResponseOutparam,
    config: &SessionConfig,
) {
    use crate::bindings::wasi::http::types::Fields;
    use crate::bindings::wasi::http::types::{OutgoingBody, OutgoingResponse};

    eprintln!("[TRANSPORT] Using SSE mode (true streaming, immediate writes)");

    let response_headers = Fields::new();
    if response_headers
        .set("content-type", &[b"text/event-stream".to_vec()])
        .is_err()
    {
        let error = TransportError::internal("Failed to set content-type");
        let response = transport_error_to_response(&error);
        ResponseOutparam::set(response_out, Ok(response));
        return;
    }
    if response_headers
        .set("cache-control", &[b"no-cache".to_vec()])
        .is_err()
    {
        let error = TransportError::internal("Failed to set cache-control");
        let response = transport_error_to_response(&error);
        ResponseOutparam::set(response_out, Ok(response));
        return;
    }

    let response = OutgoingResponse::new(response_headers);
    if response.set_status_code(200).is_err() {
        let error = TransportError::internal("Failed to set status");
        let err_response = transport_error_to_response(&error);
        ResponseOutparam::set(response_out, Ok(err_response));
        return;
    }

    let output_body = match response.body() {
        Ok(b) => b,
        Err(_) => {
            let error = TransportError::internal("Failed to get response body");
            let err_response = transport_error_to_response(&error);
            ResponseOutparam::set(response_out, Ok(err_response));
            return;
        }
    };

    // Set response FIRST to enable streaming
    ResponseOutparam::set(response_out, Ok(response));

    let output_stream = match output_body.write() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("[TRANSPORT] ERROR getting output stream");
            return;
        }
    };

    // Process request with SSE framing
    if let Err(e) = handle_mcp_request(
        request_id.clone(),
        client_request,
        protocol_version,
        session_id,
        &output_stream,
        &common::http_sse_frame(),
        config,
    ) {
        eprintln!("[TRANSPORT] ERROR during request processing: {:?}", e);
        // Write error response to SSE stream
        use crate::bindings::wasmcp::mcp_v20250618::mcp::{Error, ErrorCode, ServerMessage};
        let error_code = ErrorCode::InternalError(Error {
            code: -32603,
            message: e.message(),
            data: None,
        });
        let error_message = ServerMessage::Error((Some(request_id), error_code));
        let _ = crate::bindings::wasmcp::mcp_v20250618::server_io::send_message(
            &output_stream,
            error_message,
            &common::http_sse_frame(),
        );
    }

    // Clean up
    drop(input_stream);
    drop(body_stream);
    drop(output_stream);
    if let Err(e) = OutgoingBody::finish(output_body, None) {
        eprintln!("[TRANSPORT] ERROR finishing body: {:?}", e);
    }
    eprintln!("[TRANSPORT] SSE streaming response complete");
}

/// Handle POST request in SSE buffered mode (accumulate and flush once)
fn handle_sse_buffered_mode(
    request_id: RequestId,
    client_request: ClientRequest,
    protocol_version: String,
    session_id: Option<&str>,
    input_stream: crate::bindings::wasi::io::streams::InputStream,
    body_stream: crate::bindings::wasi::http::types::IncomingBody,
    response_out: ResponseOutparam,
    config: &SessionConfig,
) {
    use crate::bindings::wasi::http::types::Fields;
    use crate::bindings::wasi::http::types::{OutgoingBody, OutgoingResponse};
    use crate::bindings::wasmcp::mcp_v20250618::server_io;

    eprintln!("[TRANSPORT] Using SSE_Buffer mode (buffered SSE with single flush)");

    let response_headers = Fields::new();
    if response_headers
        .set("content-type", &[b"text/event-stream".to_vec()])
        .is_err()
    {
        let error = TransportError::internal("Failed to set content-type");
        let response = transport_error_to_response(&error);
        ResponseOutparam::set(response_out, Ok(response));
        return;
    }
    if response_headers
        .set("cache-control", &[b"no-cache".to_vec()])
        .is_err()
    {
        let error = TransportError::internal("Failed to set cache-control");
        let response = transport_error_to_response(&error);
        ResponseOutparam::set(response_out, Ok(response));
        return;
    }

    let response = OutgoingResponse::new(response_headers);
    if response.set_status_code(200).is_err() {
        let error = TransportError::internal("Failed to set status");
        let err_response = transport_error_to_response(&error);
        ResponseOutparam::set(response_out, Ok(err_response));
        return;
    }

    let output_body = match response.body() {
        Ok(b) => b,
        Err(_) => {
            let error = TransportError::internal("Failed to get response body");
            let err_response = transport_error_to_response(&error);
            ResponseOutparam::set(response_out, Ok(err_response));
            return;
        }
    };

    let output_stream = match output_body.write() {
        Ok(s) => s,
        Err(_) => {
            let error = TransportError::internal("Failed to get output stream");
            let err_response = transport_error_to_response(&error);
            ResponseOutparam::set(response_out, Ok(err_response));
            return;
        }
    };

    // Process request with SSE framing
    if let Err(e) = handle_mcp_request(
        request_id.clone(),
        client_request,
        protocol_version,
        session_id,
        &output_stream,
        &common::http_sse_frame(),
        config,
    ) {
        eprintln!("[TRANSPORT] ERROR during request processing: {:?}", e);
        // Write error response to SSE stream
        use crate::bindings::wasmcp::mcp_v20250618::mcp::{Error, ErrorCode, ServerMessage};
        let error_code = ErrorCode::InternalError(Error {
            code: -32603,
            message: e.message(),
            data: None,
        });
        let error_message = ServerMessage::Error((Some(request_id), error_code));
        let _ = crate::bindings::wasmcp::mcp_v20250618::server_io::send_message(
            &output_stream,
            error_message,
            &common::http_sse_frame(),
        );
    }

    // Flush buffer (single write)
    if let Err(e) = server_io::flush_buffer(&output_stream) {
        eprintln!("[TRANSPORT] ERROR flushing buffer: {:?}", e);
    }

    // Clean up - drop streams BEFORE finishing body (output_stream is child of output_body)
    drop(output_stream);
    drop(input_stream);
    drop(body_stream);
    if let Err(e) = OutgoingBody::finish(output_body, None) {
        eprintln!("[TRANSPORT] ERROR finishing body: {:?}", e);
    }

    // Set response AFTER all writes complete
    ResponseOutparam::set(response_out, Ok(response));
    eprintln!("[TRANSPORT] SSE buffered response complete");
}

/// Send result or error response for notifications, results, and errors
fn respond_with_result(result: Result<(), TransportError>, response_out: ResponseOutparam) {
    match result {
        Ok(_) => match create_accepted_response() {
            Ok(response) => ResponseOutparam::set(response_out, Ok(response)),
            Err(e) => {
                let response = transport_error_to_response(&TransportError::internal(e));
                ResponseOutparam::set(response_out, Ok(response));
            }
        },
        Err(e) => {
            let response = transport_error_to_response(&e);
            ResponseOutparam::set(response_out, Ok(response));
        }
    }
}

/// Extract session ID from request headers
fn extract_session_id_header(request: &IncomingRequest) -> Result<Option<String>, TransportError> {
    let headers = request.headers();
    let session_id_values = headers.get("mcp-session-id");

    if session_id_values.is_empty() {
        return Ok(None);
    }

    String::from_utf8(session_id_values[0].clone())
        .map(Some)
        .map_err(|_| TransportError::validation("Invalid Mcp-Session-Id header encoding"))
}

fn handle_mcp_notification(
    client_notification: ClientNotification,
    protocol_version: String,
    session_id: Option<&str>,
    config: &SessionConfig,
) -> Result<(), TransportError> {
    // Parse protocol version
    let proto_ver =
        common::parse_protocol_version(&protocol_version).map_err(TransportError::protocol)?;

    let bucket = config.get_bucket().to_string();

    // Delegate to middleware via notification context
    common::delegate_notification(
        client_notification,
        proto_ver,
        session_id,
        bucket,
        &common::plain_json_frame(),
    )
    .map_err(|e| TransportError::protocol(format!("Notification handling failed: {:?}", e)))?;

    Ok(())
}

fn handle_get(
    _request: IncomingRequest,
    _protocol_version: String,
    response_out: ResponseOutparam,
    _session_config: &SessionConfig,
) {
    eprintln!("[TRANSPORT] GET request received - returning 405 Method Not Allowed");
    match create_method_not_allowed_response(_session_config) {
        Ok(response) => {
            eprintln!("[TRANSPORT] 405 response created successfully");
            ResponseOutparam::set(response_out, Ok(response));
        }
        Err(e) => {
            eprintln!("[TRANSPORT] Error creating 405 response: {}", e);
            let error = TransportError::internal(e);
            let response = transport_error_to_response(&error);
            ResponseOutparam::set(response_out, Ok(response));
        }
    }
}

/// Handle DELETE request for session cleanup
fn handle_delete(
    request: IncomingRequest,
    response_out: ResponseOutparam,
    session_config: &SessionConfig,
) {
    // If sessions not enabled, return 405 Method Not Allowed
    if !session_config.enabled {
        match create_method_not_allowed_response(session_config) {
            Ok(response) => ResponseOutparam::set(response_out, Ok(response)),
            Err(e) => {
                let error = TransportError::internal(e);
                let response = transport_error_to_response(&error);
                ResponseOutparam::set(response_out, Ok(response));
            }
        }
        return;
    }

    // Extract session ID from header
    let session_id = match extract_session_id_header(&request) {
        Ok(Some(id)) => id,
        Ok(None) => {
            let error = TransportError::session("No session to delete");
            let response = transport_error_to_response(&error);
            ResponseOutparam::set(response_out, Ok(response));
            return;
        }
        Err(e) => {
            let response = transport_error_to_response(&e);
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
            if response.set_status_code(200).is_err() {
                let error = TransportError::internal("Failed to set status");
                let err_response = transport_error_to_response(&error);
                ResponseOutparam::set(response_out, Ok(err_response));
                return;
            }
            ResponseOutparam::set(response_out, Ok(response));
        }
        Err(SessionError::NoSuchSession) => {
            let error = TransportError::session("Session not found");
            let response = transport_error_to_response(&error);
            ResponseOutparam::set(response_out, Ok(response));
        }
        Err(_) => {
            let error = TransportError::session("Failed to delete session");
            let response = transport_error_to_response(&error);
            ResponseOutparam::set(response_out, Ok(response));
        }
    }
}

/// Validate Accept header per MCP spec
fn validate_accept_header(request: &IncomingRequest) -> Result<(), TransportError> {
    let headers = request.headers();
    let accept_values = headers.get("accept");

    if accept_values.is_empty() {
        return Err(TransportError::validation("Missing Accept header"));
    }

    let accept_str = String::from_utf8(accept_values[0].clone())
        .map_err(|_| TransportError::validation("Invalid Accept header encoding"))?;

    let has_json = accept_str.contains("application/json") || accept_str.contains("*/*");
    let has_sse = accept_str.contains("text/event-stream") || accept_str.contains("*/*");

    if !has_json || !has_sse {
        return Err(TransportError::validation(
            "Accept header must include both application/json and text/event-stream",
        ));
    }

    Ok(())
}

/// Validate MCP-Protocol-Version header
fn validate_protocol_version(request: &IncomingRequest) -> Result<String, TransportError> {
    let headers = request.headers();
    let version_values = headers.get("mcp-protocol-version");

    if version_values.is_empty() {
        // Default to 2025-03-26 for backwards compatibility
        return Ok("2025-03-26".to_string());
    }

    let version_str = String::from_utf8(version_values[0].clone())
        .map_err(|_| TransportError::validation("Invalid MCP-Protocol-Version header encoding"))?;

    match version_str.as_str() {
        "2025-06-18" | "2025-03-26" | "2024-11-05" => Ok(version_str),
        _ => Err(TransportError::protocol(format!(
            "Unsupported MCP-Protocol-Version: {}",
            version_str
        ))),
    }
}

/// Validate Origin header to prevent DNS rebinding attacks
fn validate_origin(request: &IncomingRequest) -> Result<(), TransportError> {
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
            return Err(TransportError::validation(
                "Origin header required but not provided",
            ));
        }
        return Ok(());
    } else {
        String::from_utf8(origin_values[0].clone())
            .map_err(|_| TransportError::validation("Invalid Origin header encoding"))?
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
                Err(TransportError::validation(format!(
                    "Origin '{}' not in allowed list. Set MCP_ALLOWED_ORIGINS environment variable.",
                    origin
                )))
            }
        }
        None => validate_localhost_origin(&origin),
    }
}

/// Validate localhost origin (default secure behavior)
fn validate_localhost_origin(origin: &str) -> Result<(), TransportError> {
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

    Err(TransportError::validation(format!(
        "Origin '{}' not allowed. By default, only localhost origins are permitted.",
        origin
    )))
}

/// Convert TransportError to HTTP response
fn transport_error_to_response(error: &TransportError) -> OutgoingResponse {
    let status_code = error.http_status_code();
    let error_message = error.message();

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
fn create_method_not_allowed_response(
    session_config: &SessionConfig,
) -> Result<OutgoingResponse, String> {
    // Set Allow header based on session support
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

/// Handle result response from client (bidirectional MCP)
fn handle_mcp_result(
    result_id: RequestId,
    client_result: ClientResult,
    protocol_version: String,
    session_id: Option<&str>,
    session_config: &SessionConfig,
) -> Result<(), TransportError> {
    // Parse protocol version
    let proto_ver =
        common::parse_protocol_version(&protocol_version).map_err(TransportError::protocol)?;

    // Create message context (no client-stream for results - client sending to server)
    let ctx = common::create_message_context(
        None,
        proto_ver,
        session_id,
        session_config.get_bucket(),
        &common::plain_json_frame(),
    );

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
    session_config: &SessionConfig,
) -> Result<(), TransportError> {
    // Parse protocol version
    let proto_ver =
        common::parse_protocol_version(&protocol_version).map_err(TransportError::protocol)?;

    // Create message context (no client-stream for errors - client sending to server)
    let ctx = common::create_message_context(
        None,
        proto_ver,
        session_id,
        session_config.get_bucket(),
        &common::plain_json_frame(),
    );

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
    session_config: &SessionConfig,
) {
    eprintln!("[TRANSPORT] Handling initialize request");

    // Parse protocol version
    let proto_ver = match common::parse_protocol_version(&protocol_version) {
        Ok(v) => v,
        Err(e) => {
            let error = TransportError::protocol(e);
            let response = transport_error_to_response(&error);
            ResponseOutparam::set(response_out, Ok(response));
            return;
        }
    };
    eprintln!("[TRANSPORT] Protocol version parsed: {:?}", proto_ver);

    // Get capabilities from downstream handler
    let capabilities =
        common::discover_capabilities_for_init(proto_ver, &common::plain_json_frame());
    eprintln!("[TRANSPORT] Capabilities discovered");

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
    if headers
        .set("content-type", &[b"application/json".to_vec()])
        .is_err()
    {
        let error = TransportError::internal("Failed to set content-type");
        let response = transport_error_to_response(&error);
        ResponseOutparam::set(response_out, Ok(response));
        return;
    }
    eprintln!("[TRANSPORT] Content-type header set successfully");

    // Set Mcp-Session-Id header if session was created
    if let Some(ref session_id) = new_session_id {
        eprintln!("[TRANSPORT] Setting Mcp-Session-Id header: {}", session_id);
        if headers
            .set("mcp-session-id", &[session_id.as_bytes().to_vec()])
            .is_err()
        {
            let error = TransportError::internal("Failed to set Mcp-Session-Id header");
            let response = transport_error_to_response(&error);
            ResponseOutparam::set(response_out, Ok(response));
            return;
        }
        eprintln!("[TRANSPORT] Mcp-Session-Id header set successfully");
    }

    eprintln!("[TRANSPORT] Creating OutgoingResponse");
    let response = OutgoingResponse::new(headers);
    eprintln!("[TRANSPORT] Setting status code 200");
    if response.set_status_code(200).is_err() {
        let error = TransportError::internal("Failed to set status");
        let err_response = transport_error_to_response(&error);
        ResponseOutparam::set(response_out, Ok(err_response));
        return;
    }
    eprintln!("[TRANSPORT] Status code set successfully");

    eprintln!("[TRANSPORT] Getting response body");
    let body = match response.body() {
        Ok(b) => b,
        Err(_) => {
            let error = TransportError::internal("Failed to get response body");
            let err_response = transport_error_to_response(&error);
            ResponseOutparam::set(response_out, Ok(err_response));
            return;
        }
    };
    eprintln!("[TRANSPORT] Getting output stream");
    let output_stream = match body.write() {
        Ok(s) => s,
        Err(_) => {
            let error = TransportError::internal("Failed to get output stream");
            let err_response = transport_error_to_response(&error);
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
        let error =
            TransportError::internal(format!("Failed to send initialize response: {:?}", e));
        let err_response = transport_error_to_response(&error);
        ResponseOutparam::set(response_out, Ok(err_response));
        return;
    }
    eprintln!("[TRANSPORT] Initialize response sent successfully");

    // Flush buffered data if in buffer mode
    eprintln!("[TRANSPORT] Flushing buffer...");
    if let Err(e) = server_io::flush_buffer(&output_stream) {
        eprintln!("[TRANSPORT] ERROR flushing buffer: {:?}", e);
        let error = TransportError::internal(format!("Failed to flush buffer: {:?}", e));
        let err_response = transport_error_to_response(&error);
        ResponseOutparam::set(response_out, Ok(err_response));
        return;
    }
    eprintln!("[TRANSPORT] Buffer flushed successfully");

    drop(output_stream);
    if let Err(e) = OutgoingBody::finish(body, None) {
        eprintln!("[TRANSPORT] ERROR finishing body: {:?}", e);
        let error = TransportError::internal("Failed to finish body");
        let err_response = transport_error_to_response(&error);
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
