//! DELETE request handler for session cleanup
//!
//! Handles session termination when sessions are enabled.
//! Returns 405 Method Not Allowed when sessions are disabled.

use crate::bindings::wasi::http::types::{IncomingRequest, ResponseOutparam};
use crate::config::SessionConfig;
use crate::error::TransportError;
use crate::http::{response, session, validation};
use crate::send_error;

pub fn handle_delete(
    request: IncomingRequest,
    response_out: ResponseOutparam,
    session_config: &SessionConfig,
) {
    // If sessions not enabled, return 405 Method Not Allowed
    if !session_config.enabled {
        match response::create_method_not_allowed_response(session_config) {
            Ok(response) => {
                crate::bindings::wasi::http::types::ResponseOutparam::set(
                    response_out,
                    Ok(response),
                );
            }
            Err(e) => send_error!(response_out, TransportError::internal(e)),
        }
        return;
    }

    // Extract session ID from header
    let session_id = match validation::extract_session_id_header(&request) {
        Ok(Some(id)) => id,
        Ok(None) => {
            let error = TransportError::session("No session to delete");
            send_error!(response_out, error);
        }
        Err(e) => send_error!(response_out, e),
    };

    // Delete session using session helper
    match session::delete_session_by_id(&session_id, session_config) {
        Ok(_) => {
            // Return 200 OK
            let _ = response::ResponseBuilder::new()
                .status(200)
                .build_and_send(response_out);
        }
        Err(e) => send_error!(response_out, e),
    }
}
