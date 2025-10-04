mod bindings {
    wit_bindgen::generate!({
        world: "trace-sdk",
        generate_all,
    });
}

mod span;
mod tracer;
mod export;
mod otlp;

use bindings::exports::wasi::otel_sdk::trace;
use bindings::wasi::otel_sdk::foundation;
use bindings::wasi::otel_sdk::otel_export;
use bindings::wasi::otel_sdk::context;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Main component struct implementing the trace interface
pub struct Component;

/// Generic resource registry for managing WIT resource lifecycles
///
/// This is NOT part of the WIT specification - it's our solution to handle
/// resource consumption in static methods where we need to retrieve the
/// underlying implementation data.
///
/// ## The Problem:
/// WIT resources with static methods like `finish: static func(this: T) -> R`
/// consume the resource handle, and wit-bindgen doesn't provide a way to
/// access the underlying implementation once consumed.
///
/// ## Our Solution:
/// 1. Store resource data in Arc<Mutex<T>> for shared access
/// 2. Register the Arc<Mutex<T>> in a global registry when creating resources
/// 3. Use the handle ID as the registry key
/// 4. In static methods, retrieve data from registry using the handle ID
///
/// This pattern is reusable for all OpenTelemetry signals (traces, logs, metrics)
/// and any other WIT resources that need similar lifecycle management.
struct ResourceRegistry<T> {
    resources: HashMap<u32, Arc<Mutex<T>>>,
    next_id: u32,
}

impl<T> ResourceRegistry<T> {
    fn new() -> Self {
        Self {
            resources: HashMap::new(),
            next_id: 1,
        }
    }

    fn register(&mut self, resource: Arc<Mutex<T>>) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.resources.insert(id, resource);
        id
    }

    fn get(&self, id: u32) -> Option<Arc<Mutex<T>>> {
        self.resources.get(&id).cloned()
    }

    fn remove(&mut self, id: u32) -> Option<Arc<Mutex<T>>> {
        self.resources.remove(&id)
    }
}

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
    attributes: Vec<foundation::Attribute>,
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
        attributes: Vec<foundation::Attribute>,
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
    reg.resources.insert(handle, span_data);
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
    reg.resources.insert(handle, exporter_data);
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

    fn serialize_spans(
        spans: Vec<trace::SpanData>,
        service_resource: foundation::OtelResource,
        protocol: otel_export::ExportProtocol,
    ) -> Result<Vec<u8>, String> {
        otlp::serialize_spans_to_otlp(spans, service_resource, protocol)
    }
}

bindings::export!(Component with_types_in bindings);