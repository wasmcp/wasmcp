//! Initialize request handler
//!
//! Handles the MCP initialize request which:
//! - Negotiates protocol version
//! - Creates session if enabled
//! - Discovers capabilities by probing downstream handler
//! - Returns server info and capabilities

use crate::bindings::wasi::http::types::{Fields, OutgoingResponse};
use crate::bindings::wasmcp::mcp_v20250618::mcp::ProtocolVersion;
use crate::bindings::wasmcp::mcp_v20250618::server_handler::RequestId;
use crate::capabilities::{discover_capabilities, serialize_capabilities};
use crate::config::SessionConfig;
use crate::response::write_json_rpc_response;
use crate::session::SessionManager;

/// Handle initialize request
///
/// Per MCP spec: "A server using the Streamable HTTP transport MAY assign a session ID
/// at initialization time, by including it in an Mcp-Session-Id header on the HTTP response"
///
/// # Arguments
/// * `json_rpc` - JSON-RPC initialize request
/// * `request_id` - Request ID
/// * `_existing_session_id` - Should always be None for initialize
///
/// # Returns
/// * `Ok(OutgoingResponse)` - JSON response with server info and capabilities
/// * `Err(String)` - Error message
pub fn handle_initialize_request(
    json_rpc: &serde_json::Value,
    request_id: RequestId,
    _existing_session_id: Option<String>, // Should always be None for initialize
) -> Result<OutgoingResponse, String> {
    // Parse initialize request parameters
    let params = json_rpc
        .get("params")
        .ok_or("Missing params in initialize request")?;

    let client_protocol_version = params
        .get("protocolVersion")
        .and_then(|v| v.as_str())
        .ok_or("Missing protocolVersion in initialize params")?;

    // Negotiate protocol version
    let protocol_version = match client_protocol_version {
        "2025-06-18" => ProtocolVersion::V20250618,
        "2025-03-26" => ProtocolVersion::V20250326,
        "2024-11-05" => ProtocolVersion::V20241105,
        _ => {
            // Client sent unsupported version, respond with our latest
            ProtocolVersion::V20250618
        }
    };

    // Create session if enabled
    // Per MCP spec: "A server using the Streamable HTTP transport MAY assign a session ID
    // at initialization time, by including it in an Mcp-Session-Id header on the HTTP response"
    let config = SessionConfig::from_env();
    eprintln!(
        "[INIT] Session config: enabled={}, bucket={}",
        config.enabled, config.bucket_name
    );
    let new_session_id = if config.enabled {
        eprintln!("[INIT] Sessions enabled, attempting to create session...");
        match SessionManager::initialize(&config.bucket_name) {
            Ok(session) => {
                let id = session.id().to_string();
                eprintln!("[INIT] Session created successfully: {}", id);
                // SessionManager resource owns the bucket, it will be dropped here but metadata is persisted
                drop(session);
                Some(id)
            }
            Err(e) => {
                // Session creation failed - log detailed warning and continue without session
                // Per MCP spec, sessions are OPTIONAL, so this is acceptable behavior
                // IMPORTANT: Return None here to ensure no Mcp-Session-Id header is sent to client
                use crate::session::SessionError;
                match e {
                    SessionError::Store(msg) => {
                        eprintln!(
                            "[INIT] WARNING: Session creation failed due to storage error: {}",
                            msg
                        );
                        eprintln!("[INIT] Check that:");
                        eprintln!(
                            "[INIT]   - MCP_SESSION_BUCKET='{}' is valid",
                            config.bucket_name
                        );
                        eprintln!("[INIT]   - Key-value store is properly configured");
                        eprintln!("[INIT]   - Runtime has necessary permissions");
                    }
                    SessionError::Unexpected(msg) => {
                        eprintln!(
                            "[INIT] WARNING: Session creation failed unexpectedly: {}",
                            msg
                        );
                    }
                    SessionError::NoSuchSession => {
                        // This shouldn't happen during initialize (creating new session)
                        eprintln!(
                            "[INIT] WARNING: Unexpected error - NoSuchSession during session creation"
                        );
                    }
                }
                eprintln!("[INIT] Continuing without session support for this connection");
                eprintln!("[INIT] No Mcp-Session-Id header will be sent in response");
                None // Ensures no session ID is sent to client
            }
        }
    } else {
        eprintln!("[INIT] Sessions disabled, skipping session creation");
        None
    };

    // Discover capabilities by calling downstream handler's list methods
    // Use the negotiated protocol version so discovery probes use correct version
    let capabilities = discover_capabilities(client_protocol_version);

    // Serialize capabilities before we move anything
    let capabilities_json = serialize_capabilities(&capabilities);

    // Build server info
    let server_name = "wasmcp-http-transport".to_string();
    let server_title = Some("wasmcp HTTP Transport".to_string());
    let server_version = env!("CARGO_PKG_VERSION").to_string();

    // Build initialize result
    let result = serde_json::json!({
        "protocolVersion": match protocol_version {
            ProtocolVersion::V20250618 => "2025-06-18",
            ProtocolVersion::V20250326 => "2025-03-26",
            ProtocolVersion::V20241105 => "2024-11-05",
        },
        "capabilities": capabilities_json,
        "serverInfo": {
            "name": server_name,
            "title": server_title,
            "version": server_version,
        }
    });

    // Set Mcp-Session-Id header if session was created
    // Per MCP spec: Session ID communicated via Mcp-Session-Id header
    let additional_headers = if let Some(ref session_id) = new_session_id {
        eprintln!("[INIT] Setting Mcp-Session-Id header: {}", session_id);
        let headers = Fields::new();
        headers
            .set("Mcp-Session-Id", &[session_id.as_bytes().to_vec()])
            .map_err(|_| "Failed to set Mcp-Session-Id header")?;
        eprintln!("[INIT] Header set successfully");
        Some(headers)
    } else {
        eprintln!("[INIT] No session ID to set in header");
        None
    };

    // Write JSON-RPC response with optional session header
    write_json_rpc_response(&request_id, result, additional_headers.as_ref())
}
