//! HTTP transport implementation
//!
//! Handles HTTP-specific protocol concerns:
//! - Origin validation (DNS rebinding protection)
//! - Header validation (Accept, MCP-Protocol-Version)
//! - HTTP method routing (POST, GET, DELETE)
//! - Request/response lifecycle
//!
//! Delegates I/O to http-server-io via server-io interface

mod validation;

use crate::bindings::exports::wasi::http::incoming_handler::Guest;
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
    // 1. Load session configuration once for the entire request
    let session_config = SessionConfig::from_env();

    // 2. Validate Origin header (DNS rebinding protection)
    if let Err(e) = validation::validate_origin(&request) {
        let response = transport_error_to_response(&e);
        ResponseOutparam::set(response_out, Ok(response));
        return;
    }

    // 3. Extract and validate MCP-Protocol-Version header
    let protocol_version = match validation::validate_protocol_version(&request) {
        Ok(v) => v,
        Err(e) => {
            let response = transport_error_to_response(&e);
            ResponseOutparam::set(response_out, Ok(response));
            return;
        }
    };

    // 4. Parse method and handle accordingly
    let method = request.method();

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
    // Validate Accept header per spec
    if let Err(e) = validation::validate_accept_header(&request) {
        let response = transport_error_to_response(&e);
        ResponseOutparam::set(response_out, Ok(response));
        return;
    }

    // Check for session header and validate if present
    let session_id_raw = match validation::extract_session_id_header(&request) {
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

    // Validate JWT if provided
    let identity = match validation::extract_authorization_header(&request) {
        Ok(Some(jwt)) => {
            // Import server-auth for JWT validation
            use crate::bindings::wasmcp::mcp_v20250618::server_auth;

            match server_auth::decode(&jwt) {
                Ok(claims) => {
                    eprintln!("[transport] JWT validated successfully");
                    Some(crate::bindings::wasmcp::mcp_v20250618::mcp::Identity { jwt, claims })
                }
                Err(e) => {
                    eprintln!("[transport] JWT validation failed: {:?}", e);
                    // Per user requirement: "if ANYTHIGN in the flow for auth fails,
                    // jsut move on and 'pretend' we don't need auth"
                    None
                }
            }
        }
        Ok(None) => {
            eprintln!("[transport] No Authorization header present");
            None
        }
        Err(e) => {
            eprintln!(
                "[transport] Authorization header extraction failed: {:?}",
                e
            );
            // Per user requirement: gracefully degrade on auth failure
            None
        }
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
                    identity.as_ref(),
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
                        identity.as_ref(),
                        input_stream,
                        body_stream,
                        response_out,
                        session_config,
                    )
                    .await
                }
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

#[allow(clippy::too_many_arguments)]
fn handle_mcp_request(
    request_id: RequestId,
    client_request: ClientRequest,
    protocol_version: String,
    session_id: Option<&str>,
    identity: Option<&crate::bindings::wasmcp::mcp_v20250618::mcp::Identity>,
    output_stream: &OutputStream,
    frame: &common::MessageFrame,
    config: &SessionConfig,
) -> Result<(), TransportError> {
    // Parse protocol version
    let proto_ver =
        common::parse_protocol_version(&protocol_version).map_err(TransportError::protocol)?;

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
            common::handle_ping()
                .map_err(|e| TransportError::protocol(format!("Ping failed: {:?}", e)))?;
            common::write_mcp_result(output_stream, request_id, ServerResult::Ping, frame)?;
            Ok(())
        }
        ClientRequest::LoggingSetLevel(level) => {
            let level_str = common::log_level_to_string(level);
            common::handle_set_log_level(level_str)
                .map_err(|e| TransportError::protocol(format!("SetLevel failed: {:?}", e)))?;
            common::write_mcp_result(
                output_stream,
                request_id,
                ServerResult::LoggingSetLevel,
                frame,
            )?;
            Ok(())
        }
        _ => {
            let bucket = config.get_bucket().to_string();

            // Delegate all other methods to middleware
            let result = common::delegate_to_middleware(
                request_id.clone(),
                client_request,
                proto_ver,
                session_id,
                identity,
                bucket,
                output_stream,
                frame,
            )
            .map_err(|e| {
                TransportError::protocol(format!("Middleware delegation failed: {:?}", e))
            })?;

            // Write result via server-io (handles SSE formatting)
            common::write_mcp_result(output_stream, request_id, result, frame)?;
            Ok(())
        }
    }
}

/// Handle POST request in JSON mode (single buffered response)
#[allow(clippy::too_many_arguments)]
fn handle_json_mode(
    request_id: RequestId,
    client_request: ClientRequest,
    protocol_version: String,
    session_id: Option<&str>,
    identity: Option<&crate::bindings::wasmcp::mcp_v20250618::mcp::Identity>,
    input_stream: crate::bindings::wasi::io::streams::InputStream,
    body_stream: crate::bindings::wasi::http::types::IncomingBody,
    response_out: ResponseOutparam,
    config: &SessionConfig,
) {
    use crate::bindings::wasi::http::types::Fields;
    use crate::bindings::wasi::http::types::{OutgoingBody, OutgoingResponse};
    use crate::bindings::wasmcp::mcp_v20250618::server_io;

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
        identity,
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
}

/// Handle POST request in SSE streaming mode (async writes with yielding)
///
/// Mimics Spin SDK's async streaming pattern to avoid stream budget exhaustion:
/// - Writes with check_write() to respect backpressure
/// - Yields to async executor between writes via .await
/// - Allows channel buffer to drain between messages
#[allow(clippy::too_many_arguments)]
async fn handle_sse_streaming_mode(
    request_id: RequestId,
    client_request: ClientRequest,
    protocol_version: String,
    session_id: Option<&str>,
    identity: Option<&crate::bindings::wasmcp::mcp_v20250618::mcp::Identity>,
    input_stream: crate::bindings::wasi::io::streams::InputStream,
    body_stream: crate::bindings::wasi::http::types::IncomingBody,
    response_out: ResponseOutparam,
    config: &SessionConfig,
) {
    use crate::bindings::wasi::http::types::Fields;
    use crate::bindings::wasi::http::types::{OutgoingBody, OutgoingResponse};

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
        identity,
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
    match create_method_not_allowed_response(_session_config) {
        Ok(response) => {
            ResponseOutparam::set(response_out, Ok(response));
        }
        Err(e) => {
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
    let session_id = match validation::extract_session_id_header(&request) {
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
        None,
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
        None,
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

    // Get capabilities from downstream handler
    let capabilities =
        common::discover_capabilities_for_init(proto_ver, &common::plain_json_frame());

    // Create session if enabled
    let new_session_id = if session_config.enabled {
        let bucket = session_config.get_bucket();

        session_initialize(bucket).ok()
    } else {
        None
    };

    // Create plain JSON response with optional session header
    let headers = Fields::new();
    if headers
        .set("content-type", &[b"application/json".to_vec()])
        .is_err()
    {
        let error = TransportError::internal("Failed to set content-type");
        let response = transport_error_to_response(&error);
        ResponseOutparam::set(response_out, Ok(response));
        return;
    }

    // Set Mcp-Session-Id header if session was created
    if let Some(ref session_id) = new_session_id
        && headers
            .set("mcp-session-id", &[session_id.as_bytes().to_vec()])
            .is_err()
    {
        let error = TransportError::internal("Failed to set Mcp-Session-Id header");
        let response = transport_error_to_response(&error);
        ResponseOutparam::set(response_out, Ok(response));
        return;
    }

    let response = OutgoingResponse::new(headers);
    if response.set_status_code(200).is_err() {
        let error = TransportError::internal("Failed to set status");
        let err_response = transport_error_to_response(&error);
        ResponseOutparam::set(response_out, Ok(err_response));
        return;
    }

    let body = match response.body() {
        Ok(b) => b,
        Err(_) => {
            let error = TransportError::internal("Failed to get response body");
            let err_response = transport_error_to_response(&error);
            ResponseOutparam::set(response_out, Ok(err_response));
            return;
        }
    };
    let output_stream = match body.write() {
        Ok(s) => s,
        Err(_) => {
            let error = TransportError::internal("Failed to get output stream");
            let err_response = transport_error_to_response(&error);
            ResponseOutparam::set(response_out, Ok(err_response));
            return;
        }
    };

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

    // Flush buffered data if in buffer mode
    if let Err(e) = server_io::flush_buffer(&output_stream) {
        eprintln!("[TRANSPORT] ERROR flushing buffer: {:?}", e);
        let error = TransportError::internal(format!("Failed to flush buffer: {:?}", e));
        let err_response = transport_error_to_response(&error);
        ResponseOutparam::set(response_out, Ok(err_response));
        return;
    }

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
