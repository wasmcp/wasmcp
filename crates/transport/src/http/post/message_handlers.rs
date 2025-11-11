//! MCP message handlers for POST requests
//!
//! Handles different MCP message types:
//! - Requests (method invocations)
//! - Notifications (one-way messages)
//! - Results (responses from client)
//! - Errors (error responses from client)

use crate::bindings::wasi::http::types::ResponseOutparam;
use crate::bindings::wasi::io::streams::OutputStream;
use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    ClientNotification, ClientRequest, ClientResult, ErrorCode, RequestId, ServerResult,
};
use crate::bindings::wasmcp::mcp_v20250618::server_handler::handle;
use crate::common;
use crate::config::SessionConfig;
use crate::error::TransportError;
use crate::http::response;
use crate::send_error;

/// Handle MCP request message
///
/// Routes requests to appropriate handlers:
/// - Initialize: Should never reach here (handled separately)
/// - Ping: Transport-level health check
/// - LoggingSetLevel: Transport-level logging config
/// - All others: Delegated to middleware
#[allow(clippy::too_many_arguments)]
pub fn handle_mcp_request(
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
                None, // HTTP context not available in this path
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

/// Handle MCP notification message
///
/// Notifications are one-way messages that don't expect a response.
/// Delegates to middleware for processing.
pub fn handle_mcp_notification(
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
        None, // HTTP context not available for notifications
    )
    .map_err(|e| TransportError::protocol(format!("Notification handling failed: {:?}", e)))?;

    Ok(())
}

/// Handle result response from client (bidirectional MCP)
///
/// In bidirectional MCP, clients can send responses to server-initiated requests.
pub fn handle_mcp_result(
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
        None, // HTTP context not available for results
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
///
/// In bidirectional MCP, clients can send error responses to server-initiated requests.
pub fn handle_mcp_error(
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
        None, // HTTP context not available for errors
    );

    // Create client message
    let message =
        crate::bindings::wasmcp::mcp_v20250618::mcp::ClientMessage::Error((error_id, error_code));

    // Delegate to imported server-handler (should return None for errors from client)
    handle(&ctx, message);
    Ok(())
}

/// Send result or error response for notifications, results, and errors
///
/// Used for one-way messages that return 202 Accepted on success.
pub fn respond_with_result(result: Result<(), TransportError>, response_out: ResponseOutparam) {
    match result {
        Ok(_) => match response::create_accepted_response() {
            Ok(response) => {
                crate::bindings::wasi::http::types::ResponseOutparam::set(
                    response_out,
                    Ok(response),
                );
            }
            Err(e) => send_error!(response_out, TransportError::internal(e)),
        },
        Err(e) => send_error!(response_out, e),
    }
}
