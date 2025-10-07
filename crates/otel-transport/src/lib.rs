//! HTTP transport implementation for OpenTelemetry SDK.

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        world: "otel-transport",
        generate_all,
    });
}

use bindings::exports::wasi::otel_sdk::http_transport::{
    Guest as HttpTransportGuest, HttpConfig, HttpTransportError,
};
use bindings::exports::wasi::otel_sdk::transport::{
    ContentType, ExporterTransport, ExportResult, Guest as TransportGuest, GuestExporterTransport,
    SignalType,
};

use std::cell::RefCell;

mod auth;
mod http_client;

pub struct Component;

/// Transport interface implementation
impl TransportGuest for Component {
    type ExporterTransport = HttpTransportImpl;
}

/// HTTP transport interface implementation
impl HttpTransportGuest for Component {
    fn create_http_transport(config: HttpConfig) -> Result<ExporterTransport, HttpTransportError> {
        // Validate endpoint is not empty
        if config.endpoint.is_empty() {
            return Err(HttpTransportError::EmptyEndpoint);
        }

        // Validate endpoint URL format
        if !config.endpoint.starts_with("http://") && !config.endpoint.starts_with("https://") {
            return Err(HttpTransportError::InvalidEndpoint);
        }

        // Validate timeout is non-zero
        if config.timeout_ms == 0 {
            return Err(HttpTransportError::InvalidTimeout);
        }

        // All validation passed - create the implementation and wrap it in the WIT resource
        let impl_instance = HttpTransportImpl {
            config: RefCell::new(config),
        };

        Ok(ExporterTransport::new(impl_instance))
    }
}

/// HTTP transport implementation for OTLP export
pub struct HttpTransportImpl {
    config: RefCell<HttpConfig>,
}

impl GuestExporterTransport for HttpTransportImpl {
    /// Send OTLP data via HTTP transport
    fn send(&self, signal_type: SignalType, otlp_payload: Vec<u8>, content_type: ContentType) -> ExportResult {
        http_client::send_otlp_request(
            &self.config.borrow(),
            signal_type,
            &otlp_payload,
            content_type,
        )
    }

    /// Force flush any buffered data
    fn flush(&self) -> bool {
        // No internal buffering in this implementation
        true
    }

    /// Shutdown transport and release resources
    fn shutdown(this: ExporterTransport) -> bool {
        // Clean up any resources (currently none)
        drop(this);
        true
    }
}

bindings::export!(Component with_types_in bindings);