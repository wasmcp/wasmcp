//! GET request handler
//!
//! GET requests are used for OAuth 2.0 discovery endpoints:
//! - /.well-known/oauth-protected-resource (RFC 9728)
//! - /.well-known/oauth-authorization-server (RFC 8414)
//! - /.well-known/openid-configuration (OIDC Discovery)
//!
//! All other GET requests return 405 Method Not Allowed.

use crate::bindings::wasi::http::types::{IncomingRequest, ResponseOutparam};
use crate::config::SessionConfig;
use crate::error::TransportError;
use crate::http::{discovery, response};
use crate::send_error;

pub fn handle_get(
    request: IncomingRequest,
    _protocol_version: String,
    response_out: ResponseOutparam,
    session_config: &SessionConfig,
) {
    // Get request path
    let path = match request.path_with_query() {
        Some(p) => p,
        None => {
            eprintln!("[transport:get] No path in request, returning 405");
            return send_method_not_allowed(response_out, session_config);
        }
    };

    eprintln!("[transport:get] Request path: {}", path);

    // MCP Spec: Support both /.well-known/oauth-protected-resource and
    // /.well-known/oauth-protected-resource/mcp (same for all discovery endpoints)
    // Normalize by stripping /mcp suffix if present
    let normalized_path = path.strip_suffix("/mcp").unwrap_or(&path);

    if path != normalized_path {
        eprintln!(
            "[transport:get] Normalized path: {} -> {}",
            path, normalized_path
        );
    }

    // Route discovery endpoints (both with and without /mcp suffix)
    match normalized_path {
        "/.well-known/oauth-protected-resource" => {
            discovery::handle_protected_resource_metadata(&request, response_out);
        }
        "/.well-known/oauth-authorization-server" => {
            discovery::handle_authorization_server_metadata(&request, response_out);
        }
        "/.well-known/openid-configuration" => {
            discovery::handle_openid_configuration(&request, response_out);
        }
        _ => {
            eprintln!("[transport:get] Path does not match discovery endpoints, returning 405");
            send_method_not_allowed(response_out, session_config);
        }
    }
}

/// Send 405 Method Not Allowed response
fn send_method_not_allowed(response_out: ResponseOutparam, session_config: &SessionConfig) {
    match response::create_method_not_allowed_response(session_config) {
        Ok(response) => {
            crate::bindings::wasi::http::types::ResponseOutparam::set(response_out, Ok(response));
        }
        Err(e) => send_error!(response_out, TransportError::internal(e)),
    }
}
