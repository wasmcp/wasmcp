//! HTTP method handlers (POST, GET, DELETE)

use crate::bindings::wasi::http::types::{Fields, IncomingRequest, OutgoingResponse};
use crate::bindings::wasmcp::mcp_v20250618::server_handler::Session as SessionInfo;
use crate::config::SessionConfig;
use crate::handlers::{
    handle_json_rpc_notification, handle_json_rpc_request, handle_json_rpc_response,
};
use crate::response::{create_method_not_allowed_response, read_request_body};
use crate::session::{validate_session_id_format, SessionError, SessionManager};
use crate::validation::{extract_session_id, validate_accept_header};

/// Handle HTTP POST requests
///
/// POST is the primary method for sending JSON-RPC messages to the server.
/// Per MCP spec, the client MUST include an Accept header listing both
/// application/json and text/event-stream as supported content types.
///
/// # Arguments
/// * `request` - Incoming HTTP request
/// * `protocol_version` - MCP protocol version from header validation
///
/// # Returns
/// * `Ok(OutgoingResponse)` - SSE stream with JSON-RPC response
/// * `Err(String)` - Error message (formatted as "HTTP/{code}:message" if specific status needed)
pub fn handle_post(
    request: IncomingRequest,
    protocol_version: String,
) -> Result<OutgoingResponse, String> {
    // Validate Accept header per spec
    // Per MCP spec: "The client MUST include an Accept header, listing both application/json
    // and text/event-stream as supported content types"
    validate_accept_header(&request)?;

    // Extract session ID from header (before consuming request)
    let session_id_from_header = extract_session_id(&request)?;

    // Read request body
    let body = read_request_body(request.consume().map_err(|_| "Failed to consume request")?)?;

    // Parse JSON-RPC message
    let json_rpc: serde_json::Value =
        serde_json::from_slice(&body).map_err(|e| format!("Invalid JSON: {}", e))?;

    // Check if this is an initialize request (session validation happens differently)
    let is_initialize = json_rpc
        .get("method")
        .and_then(|m| m.as_str())
        .map(|m| m == "initialize")
        .unwrap_or(false);

    // Handle session management
    let config = SessionConfig::from_env();
    let session_info = if config.enabled {
        if is_initialize {
            // Create new session for initialize request
            let manager = SessionManager::initialize(&config.bucket_name)
                .map_err(|e| format!("Failed to create session: {:?}", e))?;
            Some(SessionInfo {
                session_id: manager.id().to_string(),
                store_id: config.bucket_name.clone(),
            })
        } else if let Some(session_id) = session_id_from_header {
            // Validate existing session
            eprintln!("[SESSION_VALIDATE] Validating session ID: {}", session_id);
            validate_session_id_format(&session_id)
                .map_err(|e| format!("Invalid session ID: {:?}", e))?;

            eprintln!(
                "[SESSION_VALIDATE] Opening session with bucket: '{}'",
                config.bucket_name
            );
            let manager = SessionManager::open(&config.bucket_name, &session_id).map_err(|e| {
                eprintln!("[SESSION_VALIDATE] Failed to open session: {:?}", e);
                match e {
                    SessionError::NoSuchSession => "HTTP/404:Session not found".to_string(),
                    _ => format!("HTTP/500:Session error: {:?}", e),
                }
            })?;

            eprintln!("[SESSION_VALIDATE] Session opened, checking termination status");
            // Check if session is terminated
            if manager
                .is_terminated()
                .map_err(|e| format!("HTTP/500:{:?}", e))?
            {
                eprintln!("[SESSION_VALIDATE] Session is terminated");
                return Err("HTTP/404:Session terminated".to_string());
            }

            eprintln!("[SESSION_VALIDATE] Session is valid");
            Some(SessionInfo {
                session_id,
                store_id: config.bucket_name.clone(),
            })
        } else {
            None
        }
    } else {
        None
    };

    // Determine message type (request, notification, or response)
    if json_rpc.get("method").is_some() {
        // It's a request or notification
        if let Some(id) = json_rpc.get("id") {
            // Request - handle and return SSE stream
            handle_json_rpc_request(&json_rpc, id, protocol_version, session_info)
        } else {
            // Notification - accept and return 202
            handle_json_rpc_notification(&json_rpc, protocol_version)
        }
    } else if json_rpc.get("result").is_some() || json_rpc.get("error").is_some() {
        // It's a response (from client to server)
        handle_json_rpc_response(&json_rpc, protocol_version)
    } else {
        Err("Invalid JSON-RPC message".to_string())
    }
}

/// Handle HTTP GET requests
///
/// In stateless mode, we don't support GET (no persistent SSE streams).
/// The spec allows servers to return 405 Method Not Allowed.
pub fn handle_get(
    _request: IncomingRequest,
    _protocol_version: String,
) -> Result<OutgoingResponse, String> {
    create_method_not_allowed_response()
}

/// Handle HTTP DELETE requests
///
/// Per MCP spec: "Clients that no longer need a particular session (e.g., because
/// the user is leaving the client application) SHOULD send an HTTP DELETE to the
/// MCP endpoint with the Mcp-Session-Id header, to explicitly terminate the session."
///
/// # Returns
/// * `Ok(OutgoingResponse)` - 200 OK if session deleted
/// * `Err("HTTP/404:...")` - Session not found
/// * `Err("HTTP/405:...")` - Sessions not enabled
/// * `Err("HTTP/500:...")` - Failed to delete session
pub fn handle_delete(request: IncomingRequest) -> Result<OutgoingResponse, String> {
    let config = SessionConfig::from_env();

    // If sessions not enabled, return 405 Method Not Allowed
    if !config.enabled {
        return create_method_not_allowed_response();
    }

    // Extract session ID from header
    let session_id = match extract_session_id(&request)? {
        Some(id) => id,
        None => {
            // No session ID provided - return 404 Not Found
            // Sessions are optional per MCP spec, missing session is not a bad request
            return Err("HTTP/404:No session to delete".to_string());
        }
    };

    // Open session and delete it
    match SessionManager::open(&config.bucket_name, &session_id) {
        Ok(session) => {
            // Delete the session entirely (calls bucket.delete on metadata)
            match session.delete() {
                Ok(_) => {
                    // Return 200 OK
                    let response = OutgoingResponse::new(Fields::new());
                    response
                        .set_status_code(200)
                        .map_err(|_| "Failed to set status")?;
                    Ok(response)
                }
                Err(_) => {
                    // Failed to delete - return 500 Internal Server Error
                    Err("HTTP/500:Failed to delete session".to_string())
                }
            }
        }
        Err(SessionError::NoSuchSession) => {
            // Session doesn't exist - return 404 Not Found
            Err("HTTP/404:Session not found".to_string())
        }
        Err(_) => {
            // Other errors - return 404 Not Found
            Err("HTTP/404:Failed to open session".to_string())
        }
    }
}
