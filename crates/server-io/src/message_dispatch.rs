//! Message type detection and dispatching
//!
//! Examines JSON-RPC structure to determine message type:
//! - Request (has "id" + "method")
//! - Notification (has "method" but no "id")
//! - Result (has "id" + "result")
//! - Error (has "id" + "error")

use crate::bindings::exports::wasmcp::mcp_v20250618::server_io::IoError;
use crate::bindings::wasmcp::mcp_v20250618::mcp::ClientMessage;
use crate::parser;

/// Parse JSON into client-message variant
///
/// Determines message type based on JSON-RPC structure and delegates to parser module.
pub fn parse_client_message(json: &serde_json::Value) -> Result<ClientMessage, IoError> {
    // Check for "id" field to distinguish requests from notifications
    let has_id = json.get("id").is_some();

    // Check for "result" or "error" field to distinguish responses from requests
    let has_result = json.get("result").is_some();
    let has_error = json.get("error").is_some();

    if has_result {
        // This is a result response
        let id = json
            .get("id")
            .ok_or_else(|| IoError::InvalidMcp("Missing 'id' field in result".to_string()))?;
        let request_id = parser::parse_request_id(id)?;
        let client_result = parser::parse_client_result(json)?;
        Ok(ClientMessage::Result((request_id, client_result)))
    } else if has_error {
        // This is an error response
        let id = json.get("id").and_then(|id| {
            if id.is_null() {
                None
            } else {
                parser::parse_request_id(id).ok()
            }
        });
        let error = parser::parse_error(json)?;
        Ok(ClientMessage::Error((id, error)))
    } else if has_id {
        // This is a request
        let id = json.get("id").unwrap(); // We know it exists
        let request_id = parser::parse_request_id(id)?;
        let client_request = parser::parse_client_request(json)?;
        Ok(ClientMessage::Request((request_id, client_request)))
    } else {
        // This is a notification (no id field)
        let client_notification = parser::parse_client_notification(json)?;
        Ok(ClientMessage::Notification(client_notification))
    }
}
