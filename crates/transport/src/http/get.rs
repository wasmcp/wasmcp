//! GET request handler
//!
//! GET requests are not supported by the MCP protocol.
//! Returns 405 Method Not Allowed with appropriate Allow header.

use crate::bindings::wasi::http::types::{IncomingRequest, ResponseOutparam};
use crate::config::SessionConfig;
use crate::error::TransportError;
use crate::http::response;
use crate::send_error;

pub fn handle_get(
    _request: IncomingRequest,
    _protocol_version: String,
    response_out: ResponseOutparam,
    session_config: &SessionConfig,
) {
    match response::create_method_not_allowed_response(session_config) {
        Ok(response) => {
            crate::bindings::wasi::http::types::ResponseOutparam::set(response_out, Ok(response));
        }
        Err(e) => send_error!(response_out, TransportError::internal(e)),
    }
}
