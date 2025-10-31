//! Transport-level method handlers (ping, logging/setLevel)
//!
//! These are simple methods handled directly by the transport layer
//! rather than delegated to downstream handlers.

use crate::bindings::wasi::http::types::{Fields, OutgoingBody, OutgoingResponse};
use crate::bindings::wasmcp::mcp_v20250618::server_handler::RequestId;
use crate::response::write_chunked;

/// Handle ping request
///
/// Ping is a no-op - just return empty success as plain JSON.
/// This allows clients to check if the server is alive.
pub fn handle_ping_request(request_id: RequestId) -> Result<OutgoingResponse, String> {
    let headers = Fields::new();
    headers
        .set("content-type", &[b"application/json".to_vec()])
        .map_err(|_| "Failed to set content-type")?;
    let response = OutgoingResponse::new(headers);
    response
        .set_status_code(200)
        .map_err(|_| "Failed to set status")?;

    let body = response
        .body()
        .map_err(|_| "Failed to get response body")?;
    let output_stream = body
        .write()
        .map_err(|_| "Failed to get output stream")?;

    // Return empty result object
    let json_result = serde_json::json!({
        "jsonrpc": "2.0",
        "id": match &request_id {
            RequestId::Number(n) => serde_json::Value::Number(serde_json::Number::from(*n)),
            RequestId::String(s) => serde_json::Value::String(s.clone()),
        },
        "result": {}
    });

    let json_str =
        serde_json::to_string(&json_result).map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    write_chunked(&output_stream, json_str.as_bytes())?;

    drop(output_stream);
    OutgoingBody::finish(body, None).map_err(|_| "Failed to finish body")?;

    Ok(response)
}

/// Handle logging/setLevel request
///
/// In stateless transport this is a no-op as plain JSON.
/// We can't maintain logging level state across requests.
pub fn handle_set_level_request(request_id: RequestId) -> Result<OutgoingResponse, String> {
    let headers = Fields::new();
    headers
        .set("content-type", &[b"application/json".to_vec()])
        .map_err(|_| "Failed to set content-type")?;
    let response = OutgoingResponse::new(headers);
    response
        .set_status_code(200)
        .map_err(|_| "Failed to set status")?;

    let body = response
        .body()
        .map_err(|_| "Failed to get response body")?;
    let output_stream = body
        .write()
        .map_err(|_| "Failed to get output stream")?;

    // Return empty result object
    let json_result = serde_json::json!({
        "jsonrpc": "2.0",
        "id": match &request_id {
            RequestId::Number(n) => serde_json::Value::Number(serde_json::Number::from(*n)),
            RequestId::String(s) => serde_json::Value::String(s.clone()),
        },
        "result": {}
    });

    let json_str =
        serde_json::to_string(&json_result).map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    write_chunked(&output_stream, json_str.as_bytes())?;

    drop(output_stream);
    OutgoingBody::finish(body, None).map_err(|_| "Failed to finish body")?;

    Ok(response)
}
