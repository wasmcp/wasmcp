//! Error response writer
//!
//! Implements JSON-RPC 2.0 error response serialization for the MCP protocol.
//!
//! Error responses follow the format:
//! ```json
//! {
//!   "jsonrpc": "2.0",
//!   "id": <request-id>,
//!   "error": {
//!     "code": <error-code>,
//!     "message": "<error-message>",
//!     "data": <optional-data>
//!   }
//! }
//! ```

use crate::bindings::wasmcp::mcp::output::{
    finish_message, start_message, write_message_contents, IoError,
};
use crate::bindings::wasmcp::mcp::protocol::{ErrorCode, McpError};
use crate::utils::escape_json_string;

/// Write an error response.
///
/// Serializes the error to JSON-RPC 2.0 format and writes it using the output state machine.
pub fn write_error(error: McpError) -> Result<(), IoError> {
    let code = match error.code {
        ErrorCode::ParseError => -32700,
        ErrorCode::InvalidRequest => -32600,
        ErrorCode::MethodNotFound => -32601,
        ErrorCode::InvalidParams => -32602,
        ErrorCode::InternalError => -32603,
    };

    let id_json = error.id.as_ref().map(format_id);

    let mut error_obj = String::from("{");
    error_obj.push_str(&format!(r#""code":{}"#, code));
    error_obj.push_str(&format!(
        r#","message":"{}""#,
        escape_json_string(&error.message)
    ));

    if let Some(data) = &error.data {
        error_obj.push_str(&format!(r#","data":{}"#, data));
    }

    error_obj.push('}');

    let response = match id_json {
        Some(id) => {
            format!(r#"{{"jsonrpc":"2.0","id":{},"error":{}}}"#, id, error_obj)
        }
        None => {
            format!(r#"{{"jsonrpc":"2.0","id":null,"error":{}}}"#, error_obj)
        }
    };

    // Use the output state machine
    start_message()?;
    write_message_contents(&response.into_bytes())?;
    finish_message()
}

/// Format an ID value as JSON.
fn format_id(id: &crate::bindings::wasmcp::mcp::protocol::Id) -> String {
    match id {
        crate::bindings::wasmcp::mcp::protocol::Id::Number(n) => n.to_string(),
        crate::bindings::wasmcp::mcp::protocol::Id::String(s) => {
            format!(r#""{}""#, escape_json_string(s))
        }
    }
}
