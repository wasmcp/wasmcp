//! SSE mode POST handler (streaming response)
//!
//! Handles POST requests in SSE (Server-Sent Events) mode:
//! - Sets response FIRST to enable streaming
//! - Streams output incrementally
//! - Respects backpressure
//! - Async writes with yielding
//!
//! CRITICAL: Response is set before getting output stream.
//! After that point, errors cannot use send_error! (response_out consumed).

use crate::bindings::wasi::http::types::{OutgoingBody, ResponseOutparam};
use crate::bindings::wasmcp::mcp_v20250618::mcp::{Error, ErrorCode, RequestId, ServerMessage};
use crate::common;
use crate::config::TransportConfig;
use crate::http::{post::message_handlers, response};
use crate::send_error;

/// Handle POST request in SSE streaming mode (async writes with yielding)
///
/// Mimics Spin SDK's async streaming pattern to avoid stream budget exhaustion:
/// - Writes with check_write() to respect backpressure
/// - Yields to async executor between writes via .await
/// - Allows channel buffer to drain between messages
#[allow(clippy::too_many_arguments)]
pub async fn handle_sse_streaming_mode(
    request_id: RequestId,
    client_request: crate::bindings::wasmcp::mcp_v20250618::mcp::ClientRequest,
    protocol_version: String,
    session_id: Option<&str>,
    identity: Option<&crate::bindings::wasmcp::mcp_v20250618::mcp::Identity>,
    input_stream: crate::bindings::wasi::io::streams::InputStream,
    body_stream: crate::bindings::wasi::http::types::IncomingBody,
    response_out: ResponseOutparam,
    config: &TransportConfig,
    http_context: Option<crate::bindings::wasmcp::mcp_v20250618::server_auth::HttpContext>,
) {
    let response = match response::ResponseBuilder::new()
        .status(200)
        .header("content-type", b"text/event-stream")
        .header("cache-control", b"no-cache")
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

    // Set response FIRST to enable streaming
    crate::bindings::wasi::http::types::ResponseOutparam::set(response_out, Ok(response));

    let output_stream = match output_body.write() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("[TRANSPORT] ERROR getting output stream");
            return;
        }
    };

    // Process request with SSE framing
    if let Err(e) = message_handlers::handle_mcp_request(
        request_id.clone(),
        client_request,
        protocol_version,
        session_id,
        identity,
        &output_stream,
        &common::http_sse_frame(),
        config,
        http_context,
    ) {
        eprintln!("[TRANSPORT] ERROR during request processing: {:?}", e);
        // Write error response to SSE stream
        let error_code = ErrorCode::InternalError(Error {
            code: -32603,
            message: e.message(),
            data: None,
        });
        let error_message = ServerMessage::Error((Some(request_id), error_code));
        let _ = crate::bindings::wasmcp::mcp_v20250618::server_io::send_message(
            &output_stream,
            error_message,
            &common::http_sse_frame(),
        );
    }

    // Clean up
    drop(input_stream);
    drop(body_stream);
    drop(output_stream);
    if let Err(e) = OutgoingBody::finish(output_body, None) {
        eprintln!("[TRANSPORT] ERROR finishing body: {:?}", e);
    }
}
