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
//! # use otel_trace::bindings::wasi::otel_sdk::common::OtelResource;
//! # fn example(spans: Vec<SpanData>, resource: OtelResource) -> Result<(), Box<dyn std::error::Error>> {
//! // Serialize spans to OTLP protobuf format
//! let proto_bytes = serialize_spans_to_otlp(
//!     spans,
//!     resource,
//! )?;
//! # Ok(())
//! # }
//! ```

use crate::bindings::exports::wasi::otel_sdk::trace;
use crate::bindings::wasi::otel_sdk::common;
use crate::error::SerializationError;

// Use official opentelemetry-proto crate's generated types
pub(crate) mod proto {
    pub use opentelemetry_proto::tonic::*;
}

pub mod json;
mod protobuf;

/// Serializes spans to OTLP protobuf format.
///
/// This is the main entry point for OTLP serialization. It always serializes to protobuf format.
///
/// # Arguments
///
/// * `spans` - Vector of span data to serialize
/// * `service_resource` - Service resource attributes (e.g., service.name, service.version)
///
/// # Returns
///
/// - `Ok(Vec<u8>)` - Serialized OTLP protobuf bytes ready for transport
/// - `Err(SerializationError)` - Serialization failed with specific error type
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
/// # use otel_trace::bindings::wasi::otel_sdk::common::OtelResource;
/// # fn example(spans: Vec<SpanData>, resource: OtelResource) -> Result<(), Box<dyn std::error::Error>> {
/// // Serialize to protobuf
/// let proto = serialize_spans_to_otlp(
///     spans,
///     resource,
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn serialize_spans_to_otlp(
    spans: Vec<trace::SpanData>,
    service_resource: common::OtelResource,
) -> Result<Vec<u8>, SerializationError> {
    protobuf::serialize_to_protobuf(spans, service_resource)
}
