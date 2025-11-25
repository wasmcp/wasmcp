//! HTTP transport implementation
//!
//! Handles HTTP-specific protocol concerns:
//! - Origin validation (DNS rebinding protection)
//! - Header validation (Accept, MCP-Protocol-Version)
//! - HTTP method routing (POST, GET, DELETE)
//! - Request/response lifecycle
//!
//! Delegates I/O to http-server-io via server-io interface

mod delete;
pub mod discovery;
mod get;
pub mod post;
pub(crate) mod response;
mod session;
mod validation;

use crate::bindings::exports::wasi::http::incoming_handler::Guest;
use crate::bindings::wasi::http::types::{IncomingRequest, Method, ResponseOutparam};
use crate::config::TransportConfig;
use crate::error::TransportError;
use crate::send_error;

/// Default session store ID for WASI key-value storage
pub(crate) const DEFAULT_SESSION_BUCKET: &str = "";

pub struct HttpTransportGuest;

impl Guest for HttpTransportGuest {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        // Block on async handler - this bridges sync WIT trait to async implementation
        futures::executor::block_on(handle_http_request_async(request, response_out))
    }
}

async fn handle_http_request_async(request: IncomingRequest, response_out: ResponseOutparam) {
    // 1. Load session configuration once for the entire request
    let session_config = TransportConfig::from_env();

    // 2. Validate Origin header (DNS rebinding protection)
    if let Err(e) = validation::validate_origin(&request) {
        send_error!(response_out, e);
    }

    // 3. Extract and validate MCP-Protocol-Version header
    let protocol_version = match validation::validate_protocol_version(&request) {
        Ok(v) => v,
        Err(e) => send_error!(response_out, e),
    };

    // 4. Parse method and handle accordingly
    let method = request.method();

    match method {
        Method::Post => {
            post::handle_post(request, protocol_version, response_out, &session_config).await
        }
        Method::Get => get::handle_get(request, protocol_version, response_out, &session_config),
        Method::Delete => delete::handle_delete(request, response_out, &session_config),
        _ => match response::create_method_not_allowed_response(&session_config) {
            Ok(response) => ResponseOutparam::set(response_out, Ok(response)),
            Err(e) => send_error!(response_out, TransportError::internal(e)),
        },
    }
}
