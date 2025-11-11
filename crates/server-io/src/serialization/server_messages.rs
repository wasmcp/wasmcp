//! Top-level server message serialization
//!
//! Handles the main dispatch for ServerMessage variants to JSON-RPC format.

use crate::bindings::exports::wasmcp::mcp_v20250618::server_io::IoError;
use crate::bindings::wasmcp::mcp_v20250618::mcp::{ErrorCode, RequestId, ServerMessage};
use crate::serializer;

/// Serialize server-message variant to JSON-RPC
pub fn serialize_server_message(message: &ServerMessage) -> Result<serde_json::Value, IoError> {
    match message {
        ServerMessage::Request((id, request)) => {
            // Generate JSON-RPC request
            let (method, params) = super::requests::serialize_server_request(request);
            Ok(serde_json::json!({
                "jsonrpc": "2.0",
                "id": serialize_request_id(id),
                "method": method,
                "params": params
            }))
        }
        ServerMessage::Result((id, result)) => {
            // Generate JSON-RPC result response
            Ok(serializer::serialize_jsonrpc_response(id, Ok(result)))
        }
        ServerMessage::Error((id, error_code)) => {
            // Extract Error record from ErrorCode variant
            use ErrorCode::*;
            let error = match error_code {
                ParseError(e) | InvalidRequest(e) | MethodNotFound(e) | InvalidParams(e)
                | InternalError(e) | Server(e) | JsonRpc(e) | Mcp(e) => e,
            };

            // Generate JSON-RPC error response
            let mut error_obj = serde_json::Map::new();
            error_obj.insert("code".to_string(), serde_json::json!(error.code));
            error_obj.insert(
                "message".to_string(),
                serde_json::Value::String(error.message.clone()),
            );
            if let Some(ref data) = error.data {
                if let Ok(data_value) = serde_json::from_str::<serde_json::Value>(data) {
                    error_obj.insert("data".to_string(), data_value);
                }
            }

            Ok(serde_json::json!({
                "jsonrpc": "2.0",
                "id": id.as_ref().map(serialize_request_id),
                "error": error_obj
            }))
        }
        ServerMessage::Notification(notification) => {
            // Generate JSON-RPC notification
            let (method, params) = super::notifications::serialize_server_notification(notification);
            Ok(serde_json::json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": params
            }))
        }
    }
}

/// Serialize request ID to JSON value
pub fn serialize_request_id(id: &RequestId) -> serde_json::Value {
    match id {
        RequestId::String(s) => serde_json::Value::String(s.clone()),
        RequestId::Number(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
    }
}
