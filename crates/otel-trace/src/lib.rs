//! OpenTelemetry Trace SDK for WebAssembly Components.
//!
//! This crate provides a complete implementation of the OpenTelemetry tracing specification
//! as a WebAssembly component, designed to run in WAS I-compliant runtimes.
//!
//! # Overview
//!
//! The otel-trace component implements distributed tracing following OpenTelemetry standards:
//! - **W3C TraceContext**: Standard trace propagation via HTTP headers
//! - **OTLP Export**: Support for JSON and Protobuf serialization
//! - **Span Management**: Complete span lifecycle with events, attributes, and links
//! - **Sampling**: Configurable sampling strategies (AlwaysOn, AlwaysOff, TraceIdRatio, ParentBased)
//! - **Resource Limits**: Configurable limits to prevent resource exhaustion
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────┐
//! │          WASM Component Boundary               │
//! ├────────────────────────────────────────────────┤
//! │  TracerProvider                                │
//! │    ├─→ Tracer (instrumentation scope)          │
//! │    └─→ Span (active tracing operations)        │
//! │                                                 │
//! │  TraceExporter                                 │
//! │    ├─→ OTLP Serialization (JSON/Protobuf)      │
//! │    └─→ HTTP Client (via otel-transport)        │
//! └────────────────────────────────────────────────┘
//! ```
//!
//! # Key Features
//!
//! ## Distributed Tracing
//! - Create and manage spans across service boundaries
//! - Parent-child span relationships
//! - Span links for non-hierarchical relationships
//! - Automatic trace ID and span ID generation
//!
//! ## OTLP Compliance
//! - Full OTLP/JSON and OTLP/Protobuf support
//! - Compatible with Jaeger, Grafana Cloud, Datadog, etc.
//! - Correct encoding of all OTLP fields including dropped counts
//!
//! ## Resource Management
//! - Configurable limits per span (attributes, events, links)
//! - Dropped count tracking when limits exceeded
//! - Efficient batch export of spans
//!
//! ## WASM-Specific Optimizations
//! - Uses WASI random for cryptographically secure ID generation
//! - Handle-based resource management (WIT resource pattern)
//! - Minimal memory footprint via ResourceRegistry pattern
//!
//! # WIT Interface
//!
//! This component exports the `wasi:otel-sdk/trace` interface defined in `trace.wit`.
//! It implements the OpenTelemetry tracing API as WIT resources and functions.
//!
//! # Examples
//!
//! **Note**: This is a `cdylib` crate compiled to WASM. Usage examples show the
//! conceptual API - actual usage is via WIT component composition.
//!
//! ```wit
//! // In your component's world definition:
//! world my-app {
//!     import wasi:otel-sdk/trace;
//!
//!     // Your component exports...
//! }
//! ```
//!
//! # Implementation Notes
//!
//! ## Issue Resolutions
//!
//! This implementation includes fixes for several critical issues:
//! - **Issue #3**: Limits are enforced consistently across all span operations
//! - **Issue #4**: Dropped counts are tracked and exported via trace_state encoding
//! - **Issue #5**: Static factory method pattern for fallible resource construction
//! - **Issue #6**: Flags field included in JSON serialization
//! - **Issue #7**: Consolidated ResourceRegistry for handle management
//! - **Issue #10**: Structured error types replacing String errors
//!
//! ## Component Model Patterns
//!
//! - **Resource Lifecycle**: Spans use consuming `finish` static method
//! - **Handle Management**: ResourceRegistry provides guest-side resource table
//! - **Error Handling**: WIT error variants with automatic conversions
//!
//! # Related Components
//!
//! - `context-provider`: W3C TraceContext utilities and ID generation
//! - `otel-transport`: HTTP client for OTLP export
//! - `wit-resource-registry`: Generic resource handle management

mod bindings {
    wit_bindgen::generate!({
        world: "trace-sdk",
        generate_all,
    });
}

mod error;
mod span;
mod tracer;
mod export;
mod otlp;

use bindings::exports::wasi::otel_sdk::trace;
use bindings::wasi::otel_sdk::common;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use wit_resource_registry::ResourceRegistry;

/// Main component struct implementing the trace interface
pub struct Component;

/// Global registry for managing tracer instances
static TRACER_REGISTRY: Mutex<Option<TracerRegistry>> = Mutex::new(None);

/// Global registry for managing span instances
static SPAN_REGISTRY: Mutex<Option<ResourceRegistry<span::SpanInner>>> = Mutex::new(None);

/// Global registry for managing trace exporter instances
static TRACE_EXPORTER_REGISTRY: Mutex<Option<ResourceRegistry<export::TraceExporterInner>>> = Mutex::new(None);

// Future registries for other signals:
// static LOG_EXPORTER_REGISTRY: Mutex<Option<ResourceRegistry<log::LogExporterInner>>> = Mutex::new(None);
// static METRIC_EXPORTER_REGISTRY: Mutex<Option<ResourceRegistry<metric::MetricExporterInner>>> = Mutex::new(None);

struct TracerRegistry {
    tracers: HashMap<u32, TracerInfo>,
    next_id: u32,
}

struct TracerInfo {
    name: String,
    version: Option<String>,
    schema_url: Option<String>,
    attributes: Vec<common::Attribute>,
}

impl TracerRegistry {
    fn new() -> Self {
        Self {
            tracers: HashMap::new(),
            next_id: 1,
        }
    }

    fn register_tracer(
        &mut self,
        name: String,
        version: Option<String>,
        schema_url: Option<String>,
        attributes: Vec<common::Attribute>,
    ) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        self.tracers.insert(
            id,
            TracerInfo {
                name,
                version,
                schema_url,
                attributes,
            },
        );

        id
    }

    fn get_tracer_info(&self, id: u32) -> Option<&TracerInfo> {
        self.tracers.get(&id)
    }
}


/// Initialize the tracer registry on first use
fn ensure_registry() {
    let mut registry = TRACER_REGISTRY.lock().unwrap();
    if registry.is_none() {
        *registry = Some(TracerRegistry::new());
    }
}

/// Initialize the span registry on first use
fn ensure_span_registry() {
    let mut registry = SPAN_REGISTRY.lock().unwrap();
    if registry.is_none() {
        *registry = Some(ResourceRegistry::new());
    }
}

/// Register a span with a specific handle ID (from wit-bindgen)
pub fn register_span_with_handle(handle: u32, span_data: Arc<Mutex<span::SpanInner>>) {
    ensure_span_registry();
    let mut registry = SPAN_REGISTRY.lock().unwrap();
    let reg = registry.as_mut().unwrap();
    reg.insert(handle, span_data);
}

/// Get a span from the registry
pub fn get_span(handle: u32) -> Option<Arc<Mutex<span::SpanInner>>> {
    let registry = SPAN_REGISTRY.lock().unwrap();
    registry.as_ref().and_then(|r| r.get(handle))
}

/// Remove a span from the registry
pub fn remove_span(handle: u32) -> Option<Arc<Mutex<span::SpanInner>>> {
    let mut registry = SPAN_REGISTRY.lock().unwrap();
    registry.as_mut().and_then(|r| r.remove(handle))
}

/// Initialize the trace exporter registry on first use
fn ensure_trace_exporter_registry() {
    let mut registry = TRACE_EXPORTER_REGISTRY.lock().unwrap();
    if registry.is_none() {
        *registry = Some(ResourceRegistry::new());
    }
}

/// Register a trace exporter with a specific handle ID (from wit-bindgen)
pub fn register_exporter_with_handle(handle: u32, exporter_data: Arc<Mutex<export::TraceExporterInner>>) {
    ensure_trace_exporter_registry();
    let mut registry = TRACE_EXPORTER_REGISTRY.lock().unwrap();
    let reg = registry.as_mut().unwrap();
    reg.insert(handle, exporter_data);
}

/// Get a trace exporter from the registry
pub fn get_exporter(handle: u32) -> Option<Arc<Mutex<export::TraceExporterInner>>> {
    let registry = TRACE_EXPORTER_REGISTRY.lock().unwrap();
    registry.as_ref().and_then(|r| r.get(handle))
}

/// Remove a trace exporter from the registry
pub fn remove_exporter(handle: u32) -> Option<Arc<Mutex<export::TraceExporterInner>>> {
    let mut registry = TRACE_EXPORTER_REGISTRY.lock().unwrap();
    registry.as_mut().and_then(|r| r.remove(handle))
}

/// Implement the trace Guest interface
impl bindings::exports::wasi::otel_sdk::trace::Guest for Component {
    type Span = span::SpanImpl;
    type TracerProvider = tracer::TracerProviderImpl;
    type TraceExporter = export::TraceExporterImpl;

    fn serialize_spans_protobuf(
        spans: Vec<trace::SpanData>,
        service_resource: common::OtelResource,
    ) -> Result<Vec<u8>, trace::SerializationError> {
        otlp::serialize_spans_to_otlp(spans, service_resource)
            .map_err(|e| e.into())
    }

    fn serialize_spans_json(
        spans: Vec<trace::SpanData>,
        service_resource: common::OtelResource,
    ) -> Result<Vec<u8>, trace::SerializationError> {
        otlp::json::serialize_to_json(spans, service_resource)
            .map_err(|e| e.into())
    }
}

bindings::export!(Component with_types_in bindings);