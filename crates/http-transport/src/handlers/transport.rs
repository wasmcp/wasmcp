//! Transport-level method handlers (ping, logging/setLevel)
//!
//! These are simple methods handled directly by the transport layer
//! rather than delegated to downstream handlers.

use crate::bindings::wasi::http::types::OutgoingResponse;
use crate::bindings::wasmcp::mcp_v20250618::server_handler::RequestId;
use crate::response::write_json_rpc_response;

/// Handle ping request
///
/// Ping is a no-op - just return empty success as plain JSON.
/// This allows clients to check if the server is alive.
pub fn handle_ping_request(request_id: RequestId) -> Result<OutgoingResponse, String> {
    write_json_rpc_response(&request_id, serde_json::json!({}), None)
}

// TODO: Now that we have sessions, logging/setLevel should store the log level in session state
// and use it to filter which notifications are sent to the client. Currently this is a no-op.
// Implementation would:
// 1. Extract log level from request params
// 2. Store in session metadata (requires extending SessionMetadata in session.rs)
// 3. Use stored level when deciding whether to send notifications in handlers/jsonrpc.rs

/// Handle logging/setLevel request
///
/// In stateless transport this is a no-op as plain JSON.
/// We can't maintain logging level state across requests.
pub fn handle_set_level_request(request_id: RequestId) -> Result<OutgoingResponse, String> {
    write_json_rpc_response(&request_id, serde_json::json!({}), None)
}
