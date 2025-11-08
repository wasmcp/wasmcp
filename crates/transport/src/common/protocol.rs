//! Protocol version and message context utilities

use crate::bindings::wasi::io::streams::OutputStream;
use crate::bindings::wasmcp::mcp_v20250618::mcp::ProtocolVersion;
use crate::bindings::wasmcp::mcp_v20250618::server_handler::MessageContext;
use crate::bindings::wasmcp::mcp_v20250618::server_io::MessageFrame;

/// Parse protocol version string to enum
pub fn parse_protocol_version(version: &str) -> Result<ProtocolVersion, String> {
    match version {
        "2025-06-18" => Ok(ProtocolVersion::V20250618),
        "2025-03-26" => Ok(ProtocolVersion::V20250326),
        "2024-11-05" => Ok(ProtocolVersion::V20241105),
        _ => Err(format!("Unsupported protocol version: {}", version)),
    }
}

/// Convert ProtocolVersion enum to string
pub fn protocol_version_to_string(version: ProtocolVersion) -> String {
    match version {
        ProtocolVersion::V20241105 => "2024-11-05".to_string(),
        ProtocolVersion::V20250326 => "2025-03-26".to_string(),
        ProtocolVersion::V20250618 => "2025-06-18".to_string(),
    }
}

/// Convert LogLevel enum to string
pub fn log_level_to_string(level: crate::bindings::wasmcp::mcp_v20250618::mcp::LogLevel) -> String {
    use crate::bindings::wasmcp::mcp_v20250618::mcp::LogLevel;

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

/// Create session object from optional session ID and store ID
pub fn create_session(
    session_id: Option<&str>,
    store_id: &str,
) -> Option<crate::bindings::wasmcp::mcp_v20250618::mcp::Session> {
    session_id.map(|id| crate::bindings::wasmcp::mcp_v20250618::mcp::Session {
        session_id: id.to_string(),
        store_id: store_id.to_string(),
    })
}

/// Create MessageContext with common parameters
///
/// This eliminates duplication of MessageContext construction across the codebase.
pub fn create_message_context<'a>(
    client_stream: Option<&'a OutputStream>,
    protocol_version: ProtocolVersion,
    session_id: Option<&str>,
    identity: Option<&crate::bindings::wasmcp::mcp_v20250618::mcp::Identity>,
    bucket_name: &str,
    frame: &MessageFrame,
) -> MessageContext<'a> {
    MessageContext {
        client_stream,
        protocol_version: protocol_version_to_string(protocol_version),
        session: create_session(session_id, bucket_name),
        identity: identity.cloned(),
        frame: frame.clone(),
    }
}
