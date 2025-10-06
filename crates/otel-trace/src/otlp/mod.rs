//! OpenTelemetry Protocol (OTLP) serialization.
//!
//! This module handles serialization of telemetry data to OTLP format,
//! supporting both JSON and Protobuf protocols using official OTLP specifications.
//!
//! # Supported Protocols
//!
//! - **HTTP/JSON**: Uses `serde_json` for human-readable JSON encoding
//! - **HTTP/Protobuf**: Uses `prost` with official OpenTelemetry proto definitions
//! - **gRPC**: Same as HTTP/Protobuf (protocol differences handled at transport layer)
//!
//! # OTLP Compliance
//!
//! This implementation follows the OpenTelemetry Protocol specification:
//! - Uses official `opentelemetry-proto` crate for type definitions
//! - Correctly encodes all OTLP fields including dropped counts (Issue #4)
//! - Includes sampling flags in both JSON and Protobuf (Issue #6)
//! - Groups spans by instrumentation scope as required by OTLP
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────┐
//! │ serialize_spans_to_ │ ← Main entry point
//! │      otlp()         │
//! └──────────┬──────────┘
//!            │
//!      ┌─────┴─────┐
//!      │           │
//!      ▼           ▼
//! ┌────────┐  ┌──────────┐
//! │  JSON  │  │ Protobuf │
//! └────────┘  └──────────┘
//! ```
//!
//! # Examples
//!
//! ```no_run
//! # use otel_trace::otlp::serialize_spans_to_otlp;
//! # use otel_trace::bindings::exports::wasi::otel_sdk::trace::SpanData;
//! # use otel_trace::bindings::wasi::otel_sdk::foundation::OtelResource;
//! # use otel_trace::bindings::wasi::otel_sdk::otel_export::ExportProtocol;
//! # fn example(spans: Vec<SpanData>, resource: OtelResource) -> Result<(), Box<dyn std::error::Error>> {
//! // Serialize to JSON
//! let json_bytes = serialize_spans_to_otlp(
//!     spans.clone(),
//!     resource.clone(),
//!     ExportProtocol::HttpJson
//! )?;
//!
//! // Serialize to Protobuf
//! let proto_bytes = serialize_spans_to_otlp(
//!     spans,
//!     resource,
//!     ExportProtocol::HttpProtobuf
//! )?;
//! # Ok(())
//! # }
//! ```

use crate::bindings::exports::wasi::otel_sdk::trace;
use crate::bindings::wasi::otel_sdk::foundation;
use crate::bindings::wasi::otel_sdk::otel_export;
use crate::error::SerializationError;

// Use official opentelemetry-proto crate's generated types
pub(crate) mod proto {
    pub use opentelemetry_proto::tonic::*;
}

mod json;
mod protobuf;

/// Serializes spans to OTLP format based on the export protocol.
///
/// This is the main entry point for OTLP serialization. It routes to the appropriate
/// serializer (JSON or Protobuf) based on the protocol specified.
///
/// # Arguments
///
/// * `spans` - Vector of span data to serialize
/// * `service_resource` - Service resource attributes (e.g., service.name, service.version)
/// * `protocol` - Export protocol determining serialization format
///
/// # Returns
///
/// - `Ok(Vec<u8>)` - Serialized OTLP bytes ready for HTTP transport
/// - `Err(SerializationError)` - Serialization failed with specific error type
///
/// # Protocol Routing
///
/// - `HttpJson` → JSON serialization via `serde_json`
/// - `HttpProtobuf` → Protobuf serialization via `prost`
/// - `Grpc` → Protobuf serialization (same as HttpProtobuf)
///
/// # OTLP Structure
///
/// The serialized output follows the OTLP ExportTraceServiceRequest structure:
/// ```text
/// ExportTraceServiceRequest
/// └── ResourceSpans[]
///     ├── Resource (service.name, etc.)
///     ├── ScopeSpans[] (grouped by instrumentation scope)
///     │   ├── Scope (library name/version)
///     │   └── Spans[]
///     └── SchemaUrl
/// ```
///
/// # Examples
///
/// ```no_run
/// # use otel_trace::otlp::serialize_spans_to_otlp;
/// # use otel_trace::bindings::exports::wasi::otel_sdk::trace::SpanData;
/// # use otel_trace::bindings::wasi::otel_sdk::foundation::OtelResource;
/// # use otel_trace::bindings::wasi::otel_sdk::otel_export::ExportProtocol;
/// # fn example(spans: Vec<SpanData>, resource: OtelResource) -> Result<(), Box<dyn std::error::Error>> {
/// // Serialize for Grafana Cloud (HTTP/JSON)
/// let json = serialize_spans_to_otlp(
///     spans.clone(),
///     resource.clone(),
///     ExportProtocol::HttpJson
/// )?;
///
/// // Serialize for Jaeger (HTTP/Protobuf)
/// let proto = serialize_spans_to_otlp(
///     spans,
///     resource,
///     ExportProtocol::HttpProtobuf
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn serialize_spans_to_otlp(
    spans: Vec<trace::SpanData>,
    service_resource: foundation::OtelResource,
    protocol: otel_export::ExportProtocol,
) -> Result<Vec<u8>, SerializationError> {
    match protocol {
        otel_export::ExportProtocol::HttpJson => {
            json::serialize_to_json(spans, service_resource)
        }
        otel_export::ExportProtocol::HttpProtobuf | otel_export::ExportProtocol::Grpc => {
            protobuf::serialize_to_protobuf(spans, service_resource)
        }
    }
}
