//! Common transport logic shared between HTTP and stdio implementations

pub mod capability;
pub mod framing;
pub mod protocol;

use crate::bindings::wasi::io::streams::{InputStream, OutputStream};
use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    ClientMessage, ClientNotification, ClientRequest, ErrorCode, ProtocolVersion, RequestId,
    ServerMessage, ServerResult,
};
use crate::bindings::wasmcp::mcp_v20250618::server_handler::handle;
use crate::bindings::wasmcp::mcp_v20250618::server_io::{self, IoError, ReadLimit};

// Re-export commonly used items
pub use capability::discover_capabilities_for_init;
pub use framing::{
    http_read_limit, http_sse_frame, plain_json_frame, stdio_frame, stdio_read_limit,
};
pub use protocol::{create_message_context, log_level_to_string, parse_protocol_version};

// Re-export MessageFrame so it's public
pub use crate::bindings::wasmcp::mcp_v20250618::server_io::MessageFrame;

// =============================================================================
// MESSAGE TYPES
// =============================================================================

/// Parsed MCP message from the wire
#[derive(Debug)]
pub enum McpMessage {
    Request(RequestId, ClientRequest),
    Notification(ClientNotification),
    Result(
        RequestId,
        crate::bindings::wasmcp::mcp_v20250618::mcp::ClientResult,
    ),
    Error(
        Option<RequestId>,
        crate::bindings::wasmcp::mcp_v20250618::mcp::ErrorCode,
    ),
}

// =============================================================================
// MESSAGE PARSING
// =============================================================================

/// Parse incoming MCP message using server-io
///
/// Uses the new unified parse_message() interface with explicit frame parameter.
pub fn parse_mcp_message(
    input: &InputStream,
    limit: ReadLimit,
    frame: &MessageFrame,
) -> Result<McpMessage, String> {
    let client_message = server_io::parse_message(input, &limit, frame)
        .map_err(|e| format!("Failed to parse message: {:?}", e))?;

    match client_message {
        ClientMessage::Request((request_id, client_request)) => {
            Ok(McpMessage::Request(request_id, client_request))
        }
        ClientMessage::Notification(client_notification) => {
            Ok(McpMessage::Notification(client_notification))
        }
        ClientMessage::Result((result_id, client_result)) => {
            Ok(McpMessage::Result(result_id, client_result))
        }
        ClientMessage::Error((error_id, error_code)) => Ok(McpMessage::Error(error_id, error_code)),
    }
}

// =============================================================================
// MESSAGE WRITING
// =============================================================================

/// Write MCP result using server-io
///
/// Uses the new unified send_message() interface with explicit frame parameter.
pub fn write_mcp_result(
    output: &OutputStream,
    id: RequestId,
    result: ServerResult,
    frame: &MessageFrame,
) -> Result<(), IoError> {
    let message = ServerMessage::Result((id, result));
    server_io::send_message(output, message, frame)
}

/// Handle transport-level MCP method: ping
///
/// Simple health check that returns empty success (no specific result variant)
pub fn handle_ping() -> Result<(), ErrorCode> {
    Ok(())
}

/// Handle transport-level MCP method: logging/setLevel
///
/// Transport-level logging configuration (returns empty success)
pub fn handle_set_log_level(_level: String) -> Result<(), ErrorCode> {
    // No-op for now - could be implemented with env_logger or similar
    Ok(())
}

/// Delegate non-transport methods to middleware via server-handler
#[allow(clippy::too_many_arguments)]
pub fn delegate_to_middleware(
    request_id: RequestId,
    client_request: ClientRequest,
    protocol_version: ProtocolVersion,
    session_id: Option<&str>,
    identity: Option<&crate::bindings::wasmcp::mcp_v20250618::mcp::Identity>,
    bucket_name: String,
    output_stream: &OutputStream,
    frame: &MessageFrame,
) -> Result<ServerResult, ErrorCode> {
    // Create message context
    let ctx = create_message_context(
        Some(output_stream),
        protocol_version,
        session_id,
        identity,
        &bucket_name,
        frame,
    );

    // Create client message
    let message = ClientMessage::Request((request_id, client_request));

    // Delegate to imported server-handler
    match handle(&ctx, message) {
        Some(Ok(result)) => Ok(result),
        Some(Err(e)) => Err(e),
        None => Err(ErrorCode::InternalError(
            crate::bindings::wasmcp::mcp_v20250618::mcp::Error {
                code: -32603,
                message: "Handler returned None for request".to_string(),
                data: None,
            },
        )),
    }
}

/// Delegate notification to middleware via server-handler
pub fn delegate_notification(
    client_notification: ClientNotification,
    protocol_version: ProtocolVersion,
    session_id: Option<&str>,
    bucket_name: String,
    frame: &MessageFrame,
) -> Result<(), ErrorCode> {
    // Create message context (no client-stream for notifications - they're one-way)
    let ctx = create_message_context(
        None,
        protocol_version,
        session_id,
        None,
        &bucket_name,
        frame,
    );

    // Create client message
    let message = ClientMessage::Notification(client_notification);

    // Delegate to imported server-handler (should return None for notifications)
    handle(&ctx, message);
    Ok(())
}
