//! JSON mode POST handler (buffered response)
//!
//! Handles POST requests in JSON mode:
//! - Buffers all output
//! - Sends complete response at end
//! - Sets response AFTER all writes complete
//! - Single flush operation

use crate::bindings::wasi::http::types::{OutgoingBody, ResponseOutparam};
use crate::bindings::wasmcp::mcp_v20250618::mcp::{Error, ErrorCode, RequestId, ServerMessage};
use crate::common;
use crate::config::SessionConfig;
use crate::http::{post::message_handlers, response};
use crate::send_error;

#[allow(clippy::too_many_arguments)]
pub fn handle_json_mode(
    request_id: RequestId,
    client_request: crate::bindings::wasmcp::mcp_v20250618::mcp::ClientRequest,
    protocol_version: String,
    session_id: Option<&str>,
    identity: Option<&crate::bindings::wasmcp::mcp_v20250618::mcp::Identity>,
    input_stream: crate::bindings::wasi::io::streams::InputStream,
    body_stream: crate::bindings::wasi::http::types::IncomingBody,
    response_out: ResponseOutparam,
    config: &SessionConfig,
    http_context: Option<crate::bindings::wasmcp::mcp_v20250618::server_auth::HttpContext>,
) {
    use crate::bindings::wasmcp::mcp_v20250618::server_io;

    let response = match response::ResponseBuilder::new()
        .status(200)
        .header("content-type", b"application/json")
        .build()
    {
        Ok(r) => r,
        Err(e) => send_error!(response_out, e),
    };

    let output_body = match response.body() {
        Ok(b) => b,
        Err(_) => {
            let error = crate::error::TransportError::internal("Failed to get response body");
            send_error!(response_out, error);
        }
    };

    let output_stream = match output_body.write() {
        Ok(s) => s,
        Err(_) => {
            let error = crate::error::TransportError::internal("Failed to get output stream");
            send_error!(response_out, error);
        }
    };

    // Process request with plain JSON framing
    if let Err(e) = message_handlers::handle_mcp_request(
        request_id.clone(),
        client_request,
        protocol_version,
        session_id,
        identity,
        &output_stream,
        &common::plain_json_frame(),
        config,
        http_context,
    ) {
        eprintln!("[TRANSPORT] ERROR during request processing: {:?}", e);
        // Write error response to stream
        let error_code = ErrorCode::InternalError(Error {
            code: -32603,
            message: e.message(),
            data: None,
        });
        let error_message = ServerMessage::Error((Some(request_id), error_code));
        let _ = crate::bindings::wasmcp::mcp_v20250618::server_io::send_message(
            &output_stream,
            error_message,
            &common::plain_json_frame(),
        );
    }

    // Flush buffer (single write)
    if let Err(e) = server_io::flush_buffer(&output_stream) {
        eprintln!("[TRANSPORT] ERROR flushing buffer: {:?}", e);
    }

    // Clean up - drop streams BEFORE finishing body (output_stream is child of output_body)
    drop(output_stream);
    drop(input_stream);
    drop(body_stream);
    if let Err(e) = OutgoingBody::finish(output_body, None) {
        eprintln!("[TRANSPORT] ERROR finishing body: {:?}", e);
    }

    // Set response AFTER all writes complete
    crate::bindings::wasi::http::types::ResponseOutparam::set(response_out, Ok(response));
}
