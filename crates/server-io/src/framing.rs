//! Message framing utilities
//!
//! Handles adding and removing frame delimiters (prefix/suffix) from messages.
//! Supports different transport protocols (stdio: '\n', SSE: 'data: ' prefix + '\n\n' suffix).

use crate::bindings::exports::wasmcp::mcp_v20250618::server_io::{IoError, MessageFrame};
use crate::bindings::wasmcp::mcp_v20250618::mcp::ServerMessage;

/// Serialize a message to framed bytes WITHOUT writing to stream
///
/// This is exported for transport layer to use in buffered mode.
/// Returns the complete framed message ready to write.
pub fn serialize_message_to_bytes(
    message: ServerMessage,
    frame: &MessageFrame,
) -> Result<Vec<u8>, IoError> {
    // Serialize message to JSON-RPC
    let json_rpc = crate::serialization::serialize_server_message(&message)?;

    // Convert to string
    let json_str = json_rpc.to_string();

    // Apply framing
    let mut framed = Vec::new();
    framed.extend_from_slice(&frame.prefix);
    framed.extend_from_slice(json_str.as_bytes());
    framed.extend_from_slice(&frame.suffix);

    Ok(framed)
}

/// Strip framing prefix and suffix from raw bytes
pub fn strip_framing(raw: &[u8], frame: &MessageFrame) -> Result<Vec<u8>, IoError> {
    let mut data = raw;

    // Strip prefix
    if !frame.prefix.is_empty() {
        if data.starts_with(&frame.prefix) {
            data = &data[frame.prefix.len()..];
        } else {
            return Err(IoError::InvalidJsonrpc(format!(
                "Message does not start with expected prefix: {:?}",
                String::from_utf8_lossy(&frame.prefix)
            )));
        }
    }

    // Strip suffix
    if !frame.suffix.is_empty() {
        if data.ends_with(&frame.suffix) {
            data = &data[..data.len() - frame.suffix.len()];
        } else {
            return Err(IoError::InvalidJsonrpc(format!(
                "Message does not end with expected suffix: {:?}",
                String::from_utf8_lossy(&frame.suffix)
            )));
        }
    }

    Ok(data.to_vec())
}
