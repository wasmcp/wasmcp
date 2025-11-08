//! POST request handler
//!
//! POST is the primary MCP transport method. This module handles:
//! - Header validation (Accept, session, auth)
//! - Session validation and management
//! - JWT authentication (with graceful degradation)
//! - Request body stream acquisition
//! - MCP message parsing
//! - Message type routing (Request, Notification, Result, Error)
//! - Delegation to mode-specific handlers (JSON vs SSE)

pub mod initialize;
pub mod json_mode;
pub mod message_handlers;
pub mod sse_mode;

use crate::bindings::wasi::http::types::{IncomingRequest, ResponseOutparam};
use crate::bindings::wasmcp::mcp_v20250618::mcp::ClientRequest;
use crate::common;
use crate::config::{ServerMode, SessionConfig};
use crate::error::TransportError;
use crate::http::{session, validation};
use crate::send_error;

pub async fn handle_post(
    request: IncomingRequest,
    protocol_version: String,
    response_out: ResponseOutparam,
    session_config: &SessionConfig,
) {
    // Validate Accept header per spec
    if let Err(e) = validation::validate_accept_header(&request) {
        send_error!(response_out, e);
    }

    // Validate session from request headers
    let session_id = match session::validate_session_from_request(&request, session_config) {
        Ok(id) => id,
        Err(e) => send_error!(response_out, e),
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
            send_error!(response_out, error);
        }
    };
    let input_stream = match body_stream.stream() {
        Ok(s) => s,
        Err(_) => {
            let error = TransportError::internal("Failed to get input stream");
            send_error!(response_out, error);
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
            send_error!(response_out, error);
        }
    };

    match message {
        common::McpMessage::Request(request_id, client_request) => {
            // Check if it's initialize - handle specially with plain JSON
            if matches!(client_request, ClientRequest::Initialize(_)) {
                drop(input_stream);
                drop(body_stream);
                initialize::handle_initialize_request(
                    request_id,
                    client_request,
                    protocol_version,
                    response_out,
                    session_config,
                );
                return;
            }

            // Not initialize - check if session is required
            if !session::check_session_required(session_config, session_id.as_deref()) {
                drop(input_stream);
                drop(body_stream);
                let error =
                    TransportError::session("Session ID required for non-initialize requests");
                send_error!(response_out, error);
            }

            // Not initialize - delegate to mode-specific handler
            match session_config.mode {
                ServerMode::Json => json_mode::handle_json_mode(
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
                    sse_mode::handle_sse_streaming_mode(
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
            let result = message_handlers::handle_mcp_notification(
                client_notification,
                protocol_version,
                session_id.as_deref(),
                session_config,
            );
            drop(input_stream);
            drop(body_stream);
            message_handlers::respond_with_result(result, response_out);
        }
        common::McpMessage::Result(result_id, client_result) => {
            let result = message_handlers::handle_mcp_result(
                result_id,
                client_result,
                protocol_version,
                session_id.as_deref(),
                session_config,
            );
            drop(input_stream);
            drop(body_stream);
            message_handlers::respond_with_result(result, response_out);
        }
        common::McpMessage::Error(error_id, error_code) => {
            let result = message_handlers::handle_mcp_error(
                error_id,
                error_code,
                protocol_version,
                session_id.as_deref(),
                session_config,
            );
            drop(input_stream);
            drop(body_stream);
            message_handlers::respond_with_result(result, response_out);
        }
    }
}
