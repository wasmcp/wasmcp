//! OTLP transport for OpenTelemetry SDK.

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        world: "otel-transport",
        generate_all,
    });
}

use bindings::exports::wasi::otel_sdk::otel_export::{
    ClientError, ExportConfig, ExportResult, Guest, GuestHttpClient, HttpClient,
};

use std::cell::RefCell;

mod auth;
mod http_client;

pub struct Component;

impl Guest for Component {
    type HttpClient = HttpClientImpl;
}

/// HTTP client implementation for OTLP export
pub struct HttpClientImpl {
    config: RefCell<ExportConfig>,
}

impl GuestHttpClient for HttpClientImpl {
    /// Create HTTP client from export configuration with validation
    /// Validates configuration and returns error if invalid
    fn new(config: ExportConfig) -> Result<HttpClient, ClientError> {
        // Validate endpoint is not empty
        if config.endpoint.is_empty() {
            return Err(ClientError::EmptyEndpoint);
        }

        // Validate endpoint URL format
        if !config.endpoint.starts_with("http://") && !config.endpoint.starts_with("https://") {
            return Err(ClientError::InvalidEndpoint);
        }

        // Validate timeout is non-zero
        if config.timeout_ms == 0 {
            return Err(ClientError::InvalidTimeout);
        }

        // All validation passed - create the implementation and wrap it in the WIT resource
        let impl_instance = HttpClientImpl {
            config: RefCell::new(config),
        };

        Ok(HttpClient::new(impl_instance))
    }

    /// Get the configured export protocol
    fn get_protocol(&self) -> bindings::exports::wasi::otel_sdk::otel_export::ExportProtocol {
        self.config.borrow().protocol.clone()
    }

    /// Send OTLP request to specific signal endpoint
    fn send_otlp(&self, signal_path: String, otlp_payload: Vec<u8>, content_type: String) -> ExportResult {
        http_client::send_otlp_request(
            &self.config.borrow(),
            &signal_path,
            &otlp_payload,
            &content_type,
        )
    }

    /// Force flush any buffered data (if client implements internal buffering)
    fn force_flush(&self) -> bool {
        // No internal buffering in this implementation
        true
    }

    /// Shutdown HTTP client and release resources
    fn shutdown(this: HttpClient) -> bool {
        // Clean up any resources (currently none)
        drop(this);
        true
    }
}

bindings::export!(Component with_types_in bindings);