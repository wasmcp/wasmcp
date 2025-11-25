//! GET request handler
//!
//! GET requests are used for the OAuth 2.0 discovery endpoint:
//! - /.well-known/oauth-protected-resource (RFC 9728)
//!
//! All other GET requests return 405 Method Not Allowed.

use crate::bindings::wasi::http::types::{IncomingRequest, ResponseOutparam};
use crate::config::TransportConfig;
use crate::error::TransportError;
use crate::http::{discovery, response};
use crate::send_error;

pub fn handle_get(
    request: IncomingRequest,
    _protocol_version: String,
    response_out: ResponseOutparam,
    session_config: &TransportConfig,
) {
    // Get request path
    let path = match request.path_with_query() {
        Some(p) => p,
        None => {
            return send_method_not_allowed(response_out, session_config);
        }
    };

    // MCP Spec: Support both /.well-known/oauth-protected-resource and
    // /.well-known/oauth-protected-resource/mcp
    // Normalize by stripping /mcp suffix if present
    let normalized_path = path.strip_suffix("/mcp").unwrap_or(&path);

    // Route discovery endpoint (both with and without /mcp suffix)
    match normalized_path {
        "/.well-known/oauth-protected-resource" => {
            discovery::handle_protected_resource_metadata(&request, response_out);
        }
        _ => {
            send_method_not_allowed(response_out, session_config);
        }
    }
}

/// Send 405 Method Not Allowed response
fn send_method_not_allowed(response_out: ResponseOutparam, session_config: &TransportConfig) {
    match response::create_method_not_allowed_response(session_config) {
        Ok(response) => {
            crate::bindings::wasi::http::types::ResponseOutparam::set(response_out, Ok(response));
        }
        Err(e) => send_error!(response_out, TransportError::internal(e)),
    }
}
