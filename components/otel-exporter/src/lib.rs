// OpenTelemetry Exporter Component - Modular Architecture
//
// Configuration via environment variables (Spin variables):
// - otel_exporter_otlp_endpoint: OTLP endpoint URL
// - otel_exporter_otlp_headers_authorization: Authorization header (securely managed)
// - otel_service_name: Service identifier
// - otel_resource_attributes: Comma-separated key=value pairs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::cell::RefCell;

mod bindings;
mod providers;
mod protocols;
mod config;
mod variant_config;

use bindings::exports::wasmcp::otel_exporter::api::{Guest, GuestSpan, SpanBorrow, Span as ApiSpan};
use bindings::exports::wasi::otel::tracing::{SpanContext as WitSpanContext, TraceFlags};

// Use standard OpenTelemetry types
use opentelemetry::trace::{SpanKind, Status};

use providers::grafana::GrafanaProvider;
use providers::jaeger::JaegerProvider;
use providers::generic::GenericOtlpProvider;
use providers::Provider;
use protocols::otlp_http::{OtlpHttpProtocol, OtlpHttpConfig};
use protocols::Protocol;
use variant_config::get_otel_config;

/// Thread-local storage for current span context
thread_local! {
    static CURRENT_SPAN: RefCell<Option<SpanImpl>> = RefCell::new(None);
}

/// Set the current span for user components to access
pub fn set_current_span(span: Option<SpanImpl>) {
    CURRENT_SPAN.with(|current| {
        *current.borrow_mut() = span;
    });
}

// Serializable SpanContext wrapper (workaround for bitflags serde issue)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerializableSpanContext {
    pub trace_id: String,
    pub span_id: String,
    pub trace_flags: u8, // Store as raw u8 instead of TraceFlags
    pub is_remote: bool,
    pub trace_state: Vec<(String, String)>,
}

impl From<WitSpanContext> for SerializableSpanContext {
    fn from(wit_ctx: WitSpanContext) -> Self {
        SerializableSpanContext {
            trace_id: wit_ctx.trace_id,
            span_id: wit_ctx.span_id,
            trace_flags: wit_ctx.trace_flags.bits(),
            is_remote: wit_ctx.is_remote,
            trace_state: wit_ctx.trace_state,
        }
    }
}

impl From<SerializableSpanContext> for WitSpanContext {
    fn from(ctx: SerializableSpanContext) -> Self {
        WitSpanContext {
            trace_id: ctx.trace_id,
            span_id: ctx.span_id,
            trace_flags: TraceFlags::from_bits_retain(ctx.trace_flags),
            is_remote: ctx.is_remote,
            trace_state: ctx.trace_state,
        }
    }
}

// Our Span implementation with interior mutability for events
#[derive(Debug, Clone)]
pub struct SpanImpl {
    name: String,
    context: WitSpanContext,
    parent_context: Option<WitSpanContext>,
    events: RefCell<Vec<(String, u64)>>,
    start_time_nanos: u64,
    end_time_nanos: RefCell<Option<u64>>,
    attributes: HashMap<String, String>,
    kind: SpanKind,
    status: RefCell<Status>,
}

impl SpanImpl {
    // Accessor methods for the modular architecture
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn context(&self) -> &WitSpanContext {
        &self.context
    }

    pub fn parent_context(&self) -> &Option<WitSpanContext> {
        &self.parent_context
    }

    pub fn events(&self) -> std::cell::Ref<Vec<(String, u64)>> {
        self.events.borrow()
    }

    pub fn start_time_nanos(&self) -> u64 {
        self.start_time_nanos
    }

    pub fn end_time_nanos(&self) -> Option<u64> {
        *self.end_time_nanos.borrow()
    }

    pub fn attributes(&self) -> &HashMap<String, String> {
        &self.attributes
    }

    pub fn kind(&self) -> &SpanKind {
        &self.kind
    }

    pub fn status(&self) -> std::cell::Ref<Status> {
        self.status.borrow()
    }

    fn generate_span_id() -> String {
        // Generate 8-byte span ID as hex string
        uuid::Uuid::new_v4().as_u128().to_be_bytes()[8..16]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }

    fn generate_trace_id() -> String {
        // Generate 16-byte trace ID as hex string
        uuid::Uuid::new_v4().as_u128().to_be_bytes()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }

    fn current_time_nanos() -> u64 {
        // Use WASI wall clock to get real Unix timestamp
        use bindings::wasi::clocks::wall_clock;
        let datetime = wall_clock::now();
        // Convert seconds + nanoseconds to total nanoseconds
        datetime.seconds * 1_000_000_000 + datetime.nanoseconds as u64
    }

    fn from_wasi_span_data(span_data: SpanData, parent_context: Option<WasiSpanContext>) -> Self {
        println!("[OTEL] Converting WASI span data: {}", span_data.name);

        // Convert WASI span kind to OpenTelemetry SpanKind
        let kind = match span_data.span_kind {
            WasiSpanKind::Client => SpanKind::Client,
            WasiSpanKind::Server => SpanKind::Server,
            WasiSpanKind::Producer => SpanKind::Producer,
            WasiSpanKind::Consumer => SpanKind::Consumer,
            WasiSpanKind::Internal => SpanKind::Internal,
        };

        // Convert WASI status to OpenTelemetry Status
        let status = match span_data.status {
            WasiStatus::Unset => Status::Unset,
            WasiStatus::Ok => Status::Ok,
            WasiStatus::Error(desc) => Status::Error { description: desc.into() },
        };

        // Convert datetime to nanoseconds
        let start_time_nanos = Self::datetime_to_nanos(&span_data.start_time);
        let end_time_nanos = Self::datetime_to_nanos(&span_data.end_time);

        // Convert events
        let events: Vec<(String, u64)> = span_data.events.into_iter()
            .map(|event| (event.name, Self::datetime_to_nanos(&event.time)))
            .collect();

        // Convert attributes to simple string map
        let mut attributes = HashMap::new();
        for kv in span_data.attributes {
            let value_str = match kv.value {
                Value::String(s) => s,
                Value::Bool(b) => b.to_string(),
                Value::F64(f) => f.to_string(),
                Value::S64(i) => i.to_string(),
                _ => "[complex_value]".to_string(), // Simplified for arrays
            };
            attributes.insert(kv.key, value_str);
        }

        // Convert span context
        let context = WitSpanContext {
            trace_id: span_data.span_context.trace_id,
            span_id: span_data.span_context.span_id,
            trace_flags: span_data.span_context.trace_flags,
            is_remote: span_data.span_context.is_remote,
            trace_state: span_data.span_context.trace_state,
        };

        SpanImpl {
            name: span_data.name,
            context,
            parent_context,
            events: RefCell::new(events),
            start_time_nanos,
            end_time_nanos: RefCell::new(Some(end_time_nanos)),
            attributes,
            kind,
            status: RefCell::new(status),
        }
    }

    fn datetime_to_nanos(datetime: &Datetime) -> u64 {
        // Convert WASI datetime to nanoseconds since epoch
        datetime.seconds * 1_000_000_000 + datetime.nanoseconds as u64
    }
}

impl GuestSpan for SpanImpl {
    fn new(name: String, parent_context: Option<WitSpanContext>) -> Self {
        let span_id = Self::generate_span_id();
        let trace_id = parent_context
            .as_ref()
            .map(|ctx| ctx.trace_id.clone())
            .unwrap_or_else(Self::generate_trace_id);

        let context = WitSpanContext {
            trace_id,
            span_id,
            trace_flags: TraceFlags::SAMPLED,
            is_remote: false,
            trace_state: vec![],
        };

        println!("[OTEL] Creating new span: {}", name);

        SpanImpl {
            name,
            context,
            parent_context,
            events: RefCell::new(vec![]),
            start_time_nanos: Self::current_time_nanos(),
            end_time_nanos: RefCell::new(None),
            attributes: HashMap::new(),
            kind: SpanKind::Internal, // Default to internal
            status: RefCell::new(Status::Unset), // Default status
        }
    }

    fn add_event(&self, name: String) {
        let timestamp = Self::current_time_nanos();
        println!("[OTEL] Adding event '{}' to span '{}' at {}", name, self.name, timestamp);

        // Use interior mutability to add event
        self.events.borrow_mut().push((name, timestamp));
    }

    fn finish(&self) {
        let timestamp = Self::current_time_nanos();
        println!("[OTEL] Finishing span '{}' at {}", self.name, timestamp);

        // Set end time and mark as completed
        *self.end_time_nanos.borrow_mut() = Some(timestamp);
        *self.status.borrow_mut() = Status::Ok; // Default to OK, caller can set error status via events

        // Export the completed span using the modular architecture
        println!("[OTEL] Exporting completed span '{}' using modular exporter", self.name);
        OtelExporter::export_span(self);
    }
}

// WASI OTEL Tracing Implementation
use bindings::exports::wasi::otel::tracing::{
    Guest as WasiOtelGuest, SpanData, SpanContext as WasiSpanContext,
    SpanKind as WasiSpanKind, Value, Status as WasiStatus
};
use bindings::wasi::clocks::wall_clock::Datetime;

// Single component implementation that exports both interfaces
struct Component;

impl Guest for Component {
    type Span = SpanImpl;

    fn get_context(s: SpanBorrow<'_>) -> WitSpanContext {
        let span_impl = s.get::<SpanImpl>();
        span_impl.context.clone()
    }

    fn get_current_span() -> Option<ApiSpan> {
        CURRENT_SPAN.with(|current| {
            current.borrow().clone().map(|span_impl| ApiSpan::new(span_impl))
        })
    }
}

impl WasiOtelGuest for Component {
    fn on_start(span: SpanData, parent: WasiSpanContext) {
        println!("[WASI-OTEL] Span started: {} (parent: {})", span.name, parent.span_id);

        // Convert WASI span data to our internal format and export
        let span_impl = SpanImpl::from_wasi_span_data(span, Some(parent));
        println!("[WASI-OTEL] Converting and exporting span using modular exporter");
        OtelExporter::export_span(&span_impl);
    }

    fn on_end(span: SpanData) {
        println!("[WASI-OTEL] Span ended: {}", span.name);

        // For ended spans, we just need to ensure they get exported
        let span_impl = SpanImpl::from_wasi_span_data(span, None);
        println!("[WASI-OTEL] Exporting ended span using modular exporter");
        OtelExporter::export_span(&span_impl);
    }

    fn current_span_context() -> WasiSpanContext {
        // Return a default span context
        // In a real implementation, this would return the currently active span context
        WasiSpanContext {
            trace_id: SpanImpl::generate_trace_id(),
            span_id: SpanImpl::generate_span_id(),
            trace_flags: TraceFlags::SAMPLED,
            is_remote: false,
            trace_state: vec![],
        }
    }
}

bindings::export!(Component with_types_in bindings);

// Modular exporter implementation
pub struct OtelExporter;

impl OtelExporter {
    /// Export span using the variant-based provider/protocol architecture
    fn export_span(span: &SpanImpl) {
        println!("[OTEL] Starting variant-based export for span: {}", span.name());

        // Check if tracing is enabled via variant configuration
        let config = match get_otel_config() {
            Some(cfg) => cfg,
            None => {
                println!("[OTEL] No OTEL configuration provided by user component - silently skip tracing");
                return;
            }
        };

        println!("[OTEL] Using variant-based configuration");

        // Route to appropriate protocol implementation
        let protocol_config = Self::get_protocol_config(&config.protocol);
        let protocol = OtlpHttpProtocol::new(); // For now, always use OTLP HTTP

        // Serialize span using protocol
        let serialized_data = match protocol.serialize_span(span, &protocol_config) {
            Ok(data) => data,
            Err(e) => {
                println!("[OTEL] Failed to serialize span: {:?}", e);
                return;
            }
        };

        // Route to appropriate provider implementation
        Self::send_via_provider(&config.provider, &serialized_data);
    }

    fn get_protocol_config(protocol: &bindings::wasmcp::otel_exporter::otel_provider_config::OtelProtocol) -> OtlpHttpConfig {
        use bindings::wasmcp::otel_exporter::otel_provider_config::OtelProtocol;

        match protocol {
            OtelProtocol::OtlpHttp(otlp_config) => {
                OtlpHttpConfig {
                    content_type: otlp_config.content_type.clone(),
                    compression: otlp_config.compression.as_ref().map(|c| {
                        use bindings::wasmcp::otel_exporter::otel_provider_config::CompressionType;
                        match c {
                            CompressionType::Gzip => protocols::otlp_http::CompressionType::Gzip,
                            CompressionType::Deflate => protocols::otlp_http::CompressionType::Deflate,
                            CompressionType::None => protocols::otlp_http::CompressionType::None,
                        }
                    }),
                    timeout_ms: otlp_config.timeout_ms,
                }
            },
            _ => {
                println!("[OTEL] Protocol not yet implemented, using default OTLP HTTP");
                OtlpHttpConfig::default()
            }
        }
    }

    fn send_via_provider(provider: &bindings::wasmcp::otel_exporter::otel_provider_config::OtelProvider, data: &[u8]) {
        use bindings::wasmcp::otel_exporter::otel_provider_config::OtelProvider;

        match provider {
            OtelProvider::Grafana(grafana_config) => {
                println!("[OTEL] Routing to Grafana provider");
                let provider = GrafanaProvider::new();

                // Convert WIT config to internal config format
                let config = providers::grafana::GrafanaConfig {
                    endpoint: grafana_config.endpoint.clone(),
                    api_key: grafana_config.api_key.clone(),
                    org_id: grafana_config.org_id.clone(),
                    service_name: grafana_config.service_name.clone(),
                    resource_attributes: grafana_config.resource_attributes.iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                };

                if let Err(e) = provider.send_trace_data(data, &config) {
                    println!("[OTEL] Failed to send via Grafana provider: {:?}", e);
                } else {
                    println!("[OTEL] Successfully sent via Grafana provider");
                }
            },
            OtelProvider::Jaeger(jaeger_config) => {
                println!("[OTEL] Routing to Jaeger provider");
                let provider = JaegerProvider::new();

                // Convert WIT config to internal config format
                let config = bindings::wasmcp::otel_exporter::otel_provider_config::JaegerConfig {
                    endpoint: jaeger_config.endpoint.clone(),
                    username: jaeger_config.username.clone(),
                    password: jaeger_config.password.clone(),
                    service_name: jaeger_config.service_name.clone(),
                    resource_attributes: jaeger_config.resource_attributes.clone(),
                };

                if let Err(e) = provider.send_trace_data(data, &config) {
                    println!("[OTEL] Failed to send via Jaeger provider: {:?}", e);
                } else {
                    println!("[OTEL] Successfully sent via Jaeger provider");
                }
            },
            OtelProvider::GenericOtlp(generic_config) => {
                println!("[OTEL] Routing to Generic OTLP provider");
                let provider = GenericOtlpProvider::new();

                // Convert WIT config to internal config format
                let config = bindings::wasmcp::otel_exporter::otel_provider_config::GenericOtlpConfig {
                    endpoint: generic_config.endpoint.clone(),
                    headers: generic_config.headers.clone(),
                    service_name: generic_config.service_name.clone(),
                    resource_attributes: generic_config.resource_attributes.clone(),
                };

                if let Err(e) = provider.send_trace_data(data, &config) {
                    println!("[OTEL] Failed to send via Generic OTLP provider: {:?}", e);
                } else {
                    println!("[OTEL] Successfully sent via Generic OTLP provider");
                }
            },
            _ => {
                println!("[OTEL] Provider not yet implemented: {:?}", provider);
            }
        }
    }
}