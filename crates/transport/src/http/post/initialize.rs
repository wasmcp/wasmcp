//! Initialize request handler
//!
//! The initialize request is special:
//! - Always returns plain JSON (not SSE)
//! - Creates session if sessions are enabled
//! - Returns server capabilities and metadata
//! - Sets Mcp-Session-Id header if session created

use crate::bindings::wasi::http::types::{OutgoingBody, ResponseOutparam};
use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    ClientRequest, Implementation, InitializeResult, RequestId, ServerMessage, ServerResult,
};
use crate::common;
use crate::config::TransportConfig;
use crate::error::TransportError;
use crate::http::{response, session};
use crate::send_error;

pub fn handle_initialize_request(
    request_id: RequestId,
    _client_request: ClientRequest,
    protocol_version: String,
    identity: Option<&crate::bindings::wasmcp::mcp_v20250618::mcp::Identity>,
    response_out: ResponseOutparam,
    session_config: &TransportConfig,
) {
    // Parse protocol version
    let proto_ver = match common::parse_protocol_version(&protocol_version) {
        Ok(v) => v,
        Err(e) => {
            let error = TransportError::protocol(e);
            send_error!(response_out, error);
        }
    };

    // Get capabilities from downstream handler
    let capabilities =
        common::discover_capabilities_for_init(proto_ver, &common::plain_json_frame());

    // Create session if enabled
    let new_session_id = session::initialize_session(session_config);

    // Bind JWT claims to session if both exist
    // In OAuth mode, binding failure is FATAL - delete session and fail the request
    if let (Some(session_id), Some(identity)) = (&new_session_id, identity)
        && let Err(e) = session::bind_identity_to_session(session_id, identity, session_config)
    {
        eprintln!(
            "[transport:initialize] CRITICAL: Failed to bind JWT identity to session. \
             Deleting session and failing request."
        );

        // Clean up the session we just created
        let _ = session::delete_session_by_id(session_id, session_config);

        // Return error to client
        let error =
            TransportError::internal(format!("Failed to initialize session with identity: {}", e));
        send_error!(response_out, error);
    }

    // Create plain JSON response with optional session header
    let mut builder = response::ResponseBuilder::new()
        .status(200)
        .header("content-type", b"application/json");

    // Add Mcp-Session-Id header if session was created
    if let Some(ref session_id) = new_session_id {
        builder = builder.header("mcp-session-id", session_id.as_bytes());
    }

    let response = match builder.build() {
        Ok(r) => r,
        Err(e) => send_error!(response_out, e),
    };

    let body = match response.body() {
        Ok(b) => b,
        Err(_) => {
            let error = TransportError::internal("Failed to get response body");
            send_error!(response_out, error);
        }
    };
    let output_stream = match body.write() {
        Ok(s) => s,
        Err(_) => {
            let error = TransportError::internal("Failed to get output stream");
            send_error!(response_out, error);
        }
    };

    // Build InitializeResult using MCP types
    let init_result = InitializeResult {
        meta: None,
        server_info: Implementation {
            name: "wasmcp-server".to_string(),
            title: Some("wasmcp Universal Transport Server".to_string()),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        capabilities,
        protocol_version: proto_ver,
        options: None,
    };

    // Construct ServerMessage
    let server_message = ServerMessage::Result((request_id, ServerResult::Initialize(init_result)));

    // Use server-io to write the message with plain JSON framing
    use crate::bindings::wasmcp::mcp_v20250618::server_io;
    if let Err(e) = server_io::send_message(
        &output_stream,
        server_message,
        &crate::common::plain_json_frame(),
    ) {
        eprintln!("[TRANSPORT] ERROR sending initialize response: {:?}", e);
        let error =
            TransportError::internal(format!("Failed to send initialize response: {:?}", e));
        send_error!(response_out, error);
    }

    // Flush buffered data if in buffer mode
    if let Err(e) = server_io::flush_buffer(&output_stream) {
        eprintln!("[TRANSPORT] ERROR flushing buffer: {:?}", e);
        let error = TransportError::internal(format!("Failed to flush buffer: {:?}", e));
        send_error!(response_out, error);
    }

    drop(output_stream);
    if let Err(e) = OutgoingBody::finish(body, None) {
        eprintln!("[TRANSPORT] ERROR finishing body: {:?}", e);
        let error = TransportError::internal("Failed to finish body");
        send_error!(response_out, error);
    }

    crate::bindings::wasi::http::types::ResponseOutparam::set(response_out, Ok(response));
}
