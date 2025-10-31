//! HTTP transport for the Model Context Protocol (MCP)
//!
//! This transport implements the Streamable HTTP protocol per MCP spec 2025-06-18.
//! It handles JSON-RPC framing, SSE responses, Origin validation, and optional session management.
//!
//! Architecture:
//! - WASI HTTP proxy interface (incoming requests)
//! - Delegates to imported server-handler component
//! - Returns SSE streams for all responses
//! - Optional session management via WASI KV (configurable via environment)
//!
//! Environment Variables:
//! - MCP_SESSION_ENABLED: "true"/"false" (default: "false") - Enable session support
//! - MCP_SESSION_BUCKET: Bucket name (default: "") - Must be "default" if set, empty string otherwise
//! - MCP_ALLOWED_ORIGINS: Comma-separated allowed origins (default: localhost only)
//! - MCP_REQUIRE_ORIGIN: "true" to require Origin header (default: "false")

#[cfg(feature = "draft2")]
mod bindings {
    wit_bindgen::generate!({
        path: "wit-draft2",
        world: "http-transport-draft2",
        generate_all,
    });
}

#[cfg(not(feature = "draft2"))]
mod bindings {
    wit_bindgen::generate!({
        world: "http-transport",
        generate_all,
    });
}

// Internal modules
mod capabilities;
mod config;
mod handlers;
mod parser;
mod response;
mod serializer;
mod session;
mod stream_reader;
mod validation;

use bindings::exports::wasi::http::incoming_handler::Guest;
use bindings::wasi::http::types::{IncomingRequest, OutgoingResponse, ResponseOutparam};
use handlers::{handle_delete, handle_get, handle_post};
use response::create_error_response;
use validation::{validate_origin, validate_protocol_version};

/// HTTP transport implementation
struct HttpTransport;

impl Guest for HttpTransport {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        match handle_http_request(request) {
            Ok(response) => {
                ResponseOutparam::set(response_out, Ok(response));
            }
            Err(e) => {
                // Return error response
                let response = create_error_response(e);
                ResponseOutparam::set(response_out, Ok(response));
            }
        }
    }
}

/// Main HTTP request router
///
/// # Flow
/// 1. Validate Origin header (DNS rebinding protection)
/// 2. Extract and validate MCP-Protocol-Version header
/// 3. Route by HTTP method (POST/GET/DELETE)
///
/// # Arguments
/// * `request` - Incoming HTTP request
///
/// # Returns
/// * `Ok(OutgoingResponse)` - HTTP response
/// * `Err(String)` - Error message (formatted as "HTTP/{code}:message" if specific status needed)
fn handle_http_request(request: IncomingRequest) -> Result<OutgoingResponse, String> {
    // 1. Validate Origin header (DNS rebinding protection)
    validate_origin(&request)?;

    // 2. Extract and validate MCP-Protocol-Version header
    let protocol_version = validate_protocol_version(&request)?;

    // 3. Parse method and handle accordingly
    let method = request.method();

    match method {
        bindings::wasi::http::types::Method::Post => handle_post(request, protocol_version),
        bindings::wasi::http::types::Method::Get => handle_get(request, protocol_version),
        bindings::wasi::http::types::Method::Delete => handle_delete(request),
        _ => response::create_method_not_allowed_response(),
    }
}

bindings::export!(HttpTransport with_types_in bindings);
