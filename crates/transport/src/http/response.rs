//! HTTP response construction and helpers
//!
//! This module provides utilities for building HTTP responses:
//! - ResponseBuilder for fluent response construction
//! - Helper functions for common response types (errors, 405, 202)
//! - Macros to reduce boilerplate

use crate::bindings::wasi::http::types::{
    Fields, OutgoingBody, OutgoingResponse, ResponseOutparam,
};
use crate::config::SessionConfig;
use crate::error::TransportError;

/// Macro to send an error response and return early
///
/// Usage: `send_error!(response_out, error)`
#[macro_export]
macro_rules! send_error {
    ($response_out:expr, $error:expr) => {{
        let response = $crate::http::response::transport_error_to_response(&$error);
        $crate::bindings::wasi::http::types::ResponseOutparam::set($response_out, Ok(response));
        return;
    }};
}

/// Fluent builder for HTTP responses
///
/// Handles the common pattern of creating a response, setting headers and status,
/// with proper error handling at each step.
///
/// # Example
/// ```ignore
/// ResponseBuilder::new()
///     .status(200)
///     .header("content-type", b"application/json")
///     .build_and_send(response_out)?;
/// ```
pub struct ResponseBuilder {
    status: u16,
    headers: Vec<(&'static str, Vec<u8>)>,
}

impl ResponseBuilder {
    /// Create a new response builder with default 200 OK status
    pub fn new() -> Self {
        Self {
            status: 200,
            headers: Vec::new(),
        }
    }

    /// Set the HTTP status code
    pub fn status(mut self, code: u16) -> Self {
        self.status = code;
        self
    }

    /// Add a header (fluent)
    pub fn header(mut self, name: &'static str, value: &[u8]) -> Self {
        self.headers.push((name, value.to_vec()));
        self
    }

    /// Build the response, returning Result for error handling
    pub fn build(self) -> Result<OutgoingResponse, TransportError> {
        // Create headers
        let fields = Fields::new();
        for (name, value) in &self.headers {
            fields
                .set(name, std::slice::from_ref(value))
                .map_err(|_| TransportError::internal(format!("Failed to set {} header", name)))?;
        }

        // Create response with headers
        let response = OutgoingResponse::new(fields);
        response
            .set_status_code(self.status)
            .map_err(|_| TransportError::internal("Failed to set status code"))?;

        Ok(response)
    }

    /// Build and immediately send the response to ResponseOutparam
    pub fn build_and_send(self, response_out: ResponseOutparam) -> Result<(), ()> {
        match self.build() {
            Ok(response) => {
                ResponseOutparam::set(response_out, Ok(response));
                Ok(())
            }
            Err(e) => {
                let error_response = transport_error_to_response(&e);
                ResponseOutparam::set(response_out, Ok(error_response));
                Err(())
            }
        }
    }
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert TransportError to HTTP response with JSON error body
pub fn transport_error_to_response(error: &TransportError) -> OutgoingResponse {
    let status_code = error.http_status_code();
    let error_message = error.message();

    let response = OutgoingResponse::new(Fields::new());
    let _ = response.set_status_code(status_code);

    let headers = response.headers();
    let _ = headers.set("content-type", &[b"application/json".to_vec()]);

    if let Ok(body) = response.body() {
        if let Ok(stream) = body.write() {
            let error_json = serde_json::json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": {
                    "code": -32700,
                    "message": error_message
                }
            });
            let _ = stream.blocking_write_and_flush(error_json.to_string().as_bytes());
            drop(stream);
        }
        let _ = OutgoingBody::finish(body, None);
    }

    response
}

/// Create 405 Method Not Allowed response with appropriate Allow header
pub fn create_method_not_allowed_response(
    session_config: &SessionConfig,
) -> Result<OutgoingResponse, String> {
    // Set Allow header based on session support
    let allow_methods = if session_config.enabled {
        b"POST, DELETE".to_vec()
    } else {
        b"POST".to_vec()
    };

    // Create headers first
    let headers = Fields::new();
    headers
        .set("allow", &[allow_methods])
        .map_err(|_| "Failed to set allow header")?;

    // Create response with headers
    let response = OutgoingResponse::new(headers);
    response
        .set_status_code(405)
        .map_err(|_| "Failed to set status")?;

    Ok(response)
}

/// Create 202 Accepted response (for notifications, results, errors)
pub fn create_accepted_response() -> Result<OutgoingResponse, String> {
    let response = OutgoingResponse::new(Fields::new());
    response
        .set_status_code(202)
        .map_err(|_| "Failed to set status")?;
    Ok(response)
}
