//! JSON-RPC message handlers (requests, notifications, responses)

use crate::bindings::wasi::http::types::{Fields, OutgoingBody, OutgoingResponse};
use crate::bindings::wasmcp::mcp_v20250618::server_handler::{
    handle_error, handle_notification, handle_request, handle_result, ErrorCtx, NotificationCtx,
    RequestCtx, ResultCtx, Session as SessionInfo,
};
use crate::handlers::{handle_initialize_request, handle_ping_request, handle_set_level_request};
use crate::parser;
use crate::response::{parse_request_id, write_sse_response};

/// Handle JSON-RPC request
///
/// Requests have both "method" and "id" fields.
/// Per MCP spec, responses are sent as Server-Sent Events.
///
/// Transport-level methods (initialize, ping, logging/setLevel) are handled directly.
/// All other requests are delegated to the downstream handler via server-handler interface.
///
/// # Arguments
/// * `json_rpc` - Parsed JSON-RPC request
/// * `id` - Request ID value
/// * `protocol_version` - MCP protocol version
/// * `session_info` - Session information if sessions enabled
///
/// # Returns
/// * `Ok(OutgoingResponse)` - SSE stream with JSON-RPC response
/// * `Err(String)` - Error message
pub fn handle_json_rpc_request(
    json_rpc: &serde_json::Value,
    id: &serde_json::Value,
    protocol_version: String,
    session_info: Option<SessionInfo>,
) -> Result<OutgoingResponse, String> {
    // Parse request ID
    let request_id = parse_request_id(id)?;

    // Check if this is a transport-level method
    let method = json_rpc
        .get("method")
        .and_then(|m| m.as_str())
        .ok_or("Missing method field")?;

    // Handle transport-level methods directly
    match method {
        "initialize" => {
            return handle_initialize_request(
                json_rpc,
                request_id,
                session_info.as_ref().map(|s| s.session_id.clone()),
            )
        }
        "ping" => return handle_ping_request(request_id),
        "logging/setLevel" => return handle_set_level_request(request_id),
        _ => {
            // Delegate all other requests to server-handler
        }
    }

    // Parse client request from JSON
    let client_request = parser::parse_client_request(json_rpc)?;

    // Create headers FIRST
    let headers = Fields::new();
    headers
        .set("content-type", &[b"text/event-stream".to_vec()])
        .map_err(|_| "Failed to set content-type")?;
    headers
        .set("cache-control", &[b"no-cache".to_vec()])
        .map_err(|_| "Failed to set cache-control")?;
    // Note: Transfer-Encoding is managed by the WASI HTTP runtime, don't set it manually

    // Create response with headers
    let response = OutgoingResponse::new(headers);
    response
        .set_status_code(200)
        .map_err(|_| "Failed to set status")?;

    // Get body and output stream
    let body = response.body().map_err(|_| "Failed to get response body")?;
    let output_stream = body.write().map_err(|_| "Failed to get output stream")?;

    // Create context with session info if available
    let ctx = RequestCtx {
        id: request_id.clone(),
        protocol_version,
        messages: Some(&output_stream),
        session: session_info,
        user: None,
    };

    // Delegate to server-handler (may send notifications via output stream)
    let result = handle_request(&ctx, &client_request);

    // Write final JSON-RPC response to SSE stream
    write_sse_response(&output_stream, request_id, result)?;

    // Drop stream and finish body
    drop(output_stream);
    OutgoingBody::finish(body, None).map_err(|_| "Failed to finish body")?;

    Ok(response)
}

/// Handle JSON-RPC notification
///
/// Notifications have "method" but no "id" field.
/// Per MCP spec, notifications do not expect a response.
/// The server SHOULD return 202 Accepted.
///
/// # Arguments
/// * `json_rpc` - Parsed JSON-RPC notification
/// * `protocol_version` - MCP protocol version
///
/// # Returns
/// * `Ok(OutgoingResponse)` - 202 Accepted
/// * `Err(String)` - Error message
pub fn handle_json_rpc_notification(
    json_rpc: &serde_json::Value,
    protocol_version: String,
) -> Result<OutgoingResponse, String> {
    // Parse notification from JSON
    let notification = parser::parse_client_notification(json_rpc)?;

    // Create context (stateless: no session, no user identity)
    let ctx = NotificationCtx {
        protocol_version,
        session: None,
        user: None,
    };

    // Forward to server-handler (no response expected)
    handle_notification(&ctx, &notification);

    // Return 202 Accepted
    let response = OutgoingResponse::new(Fields::new());
    response
        .set_status_code(202)
        .map_err(|_| "Failed to set status")?;
    Ok(response)
}

/// Handle JSON-RPC response (client to server)
///
/// Responses from client to server contain either "result" or "error" fields plus "id".
/// Per MCP spec, these are responses to server-initiated requests.
/// The server SHOULD return 202 Accepted after forwarding to handler.
///
/// # Arguments
/// * `json_rpc` - Parsed JSON-RPC response
/// * `protocol_version` - MCP protocol version
///
/// # Returns
/// * `Ok(OutgoingResponse)` - 202 Accepted
/// * `Err(String)` - Error message
pub fn handle_json_rpc_response(
    json_rpc: &serde_json::Value,
    protocol_version: String,
) -> Result<OutgoingResponse, String> {
    // Parse response ID (required for responses)
    let id = json_rpc.get("id").ok_or("Missing id in response")?;
    let request_id = parser::parse_request_id(id)?;

    // Parse client response from JSON
    let response_result = parser::parse_client_response(json_rpc)?;

    // Forward to server-handler (no response expected)
    match response_result {
        Ok(client_result) => {
            // Success response
            let ctx = ResultCtx {
                id: request_id,
                protocol_version: protocol_version.clone(),
                session: None,
                user: None,
            };
            handle_result(&ctx, client_result);
        }
        Err(error_code) => {
            // Error response
            let ctx = ErrorCtx {
                id: Some(request_id),
                protocol_version,
                session: None,
                user: None,
            };
            handle_error(&ctx, &error_code);
        }
    }

    // Return 202 Accepted
    let response = OutgoingResponse::new(Fields::new());
    response
        .set_status_code(202)
        .map_err(|_| "Failed to set status")?;
    Ok(response)
}
