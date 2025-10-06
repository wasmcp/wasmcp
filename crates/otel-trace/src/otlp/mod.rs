//! OpenTelemetry Protocol (OTLP) serialization
//!
//! This module handles serialization of telemetry data to OTLP format.
//! Supports both JSON and Protobuf protocols using official OTLP specifications.

use crate::bindings::exports::wasi::otel_sdk::trace;
use crate::bindings::wasi::otel_sdk::foundation;
use crate::bindings::wasi::otel_sdk::otel_export;

// Use official opentelemetry-proto crate's generated types
pub(crate) mod proto {
    pub use opentelemetry_proto::tonic::*;
}

mod json;
mod protobuf;

/// Serialize spans to OTLP format based on the export protocol
pub fn serialize_spans_to_otlp(
    spans: Vec<trace::SpanData>,
    service_resource: foundation::OtelResource,
    protocol: otel_export::ExportProtocol,
) -> Result<Vec<u8>, String> {
    match protocol {
        otel_export::ExportProtocol::HttpJson => {
            json::serialize_to_json(spans, service_resource)
        }
        otel_export::ExportProtocol::HttpProtobuf | otel_export::ExportProtocol::Grpc => {
            protobuf::serialize_to_protobuf(spans, service_resource)
        }
    }
}
