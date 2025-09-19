// OpenTelemetry Exporter Component for Grafana
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
use bindings::exports::wasmcp::otel_exporter::api::{Guest, GuestSpan, SpanBorrow, Span as ApiSpan};
use bindings::exports::wasi::otel::tracing::{SpanContext as WitSpanContext, TraceFlags};

// Use standard OpenTelemetry types
use opentelemetry::trace::{SpanKind, Status};

// Use official opentelemetry-proto generated types
use opentelemetry_proto::tonic::trace::v1 as otlp_trace;
use opentelemetry_proto::tonic::common::v1 as otlp_common;
use opentelemetry_proto::tonic::resource::v1 as otlp_resource;

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
            trace_flags: wit_ctx.trace_flags.bits(), // Convert bitflags to raw bits
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
            trace_flags: TraceFlags::from_bits_retain(ctx.trace_flags), // Convert back to bitflags
            is_remote: ctx.is_remote,
            trace_state: ctx.trace_state,
        }
    }
}

// Helper function to convert SpanKind to OTLP protobuf enum
fn span_kind_to_proto(kind: &SpanKind) -> i32 {
    match kind {
        SpanKind::Internal => 1,
        SpanKind::Server => 2,
        SpanKind::Client => 3,
        SpanKind::Producer => 4,
        SpanKind::Consumer => 5,
    }
}

// Helper function to convert Status to OTLP protobuf format
fn status_to_proto(status: &Status) -> otlp_trace::Status {
    match status {
        Status::Unset => otlp_trace::Status {
            code: 0, // STATUS_CODE_UNSET
            message: "".to_string(),
        },
        Status::Ok => otlp_trace::Status {
            code: 1, // STATUS_CODE_OK
            message: "".to_string(),
        },
        Status::Error { description } => otlp_trace::Status {
            code: 2, // STATUS_CODE_ERROR
            message: description.to_string(),
        },
    }
}

// Our Span implementation with interior mutability for events
#[derive(Debug, Clone)]
pub struct SpanImpl {
    name: String,
    context: WitSpanContext,
    parent_context: Option<WitSpanContext>,
    events: RefCell<Vec<(String, u64)>>, // Use RefCell for interior mutability
    start_time_nanos: u64,
    end_time_nanos: RefCell<Option<u64>>, // Use RefCell for end time
    attributes: HashMap<String, String>,
    kind: SpanKind,
    status: RefCell<Status>, // Use RefCell for status updates
}

impl SpanImpl {
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

        // Export the completed span
        println!("[OTEL] Exporting completed span '{}' to Grafana", self.name);
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

        // Convert WASI span data to our internal format and export to Grafana
        let span_impl = SpanImpl::from_wasi_span_data(span, Some(parent));
        println!("[WASI-OTEL] Converting and exporting span to Grafana");
        OtelExporter::export_span(&span_impl);
    }

    fn on_end(span: SpanData) {
        println!("[WASI-OTEL] Span ended: {}", span.name);

        // For ended spans, we just need to ensure they get exported
        let span_impl = SpanImpl::from_wasi_span_data(span, None);
        println!("[WASI-OTEL] Exporting ended span to Grafana");
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

// Component implementation
pub struct OtelExporter;

// Grafana export functionality
impl OtelExporter {
    /// Get OTLP configuration using Spin SDK variables
    fn get_config() -> OtlpConfig {
        println!("[OTEL] Loading configuration from Spin variables...");

        // Get configuration values using proper Spin SDK variables API
        let endpoint = spin_sdk::variables::get("otel_exporter_otlp_endpoint")
            .unwrap_or_else(|_| "http://localhost:4318".to_string());
        println!("[OTEL] Endpoint: {}", endpoint);

        let service_name = spin_sdk::variables::get("otel_service_name")
            .unwrap_or_else(|_| "wasmcp-otel-exporter".to_string());
        println!("[OTEL] Service name: {}", service_name);

        // Get the authorization header securely (contains sensitive credentials)
        let auth_header = spin_sdk::variables::get("otel_exporter_otlp_headers_authorization").ok();
        println!("[OTEL] Authorization header configured: {}", auth_header.is_some());

        // Build headers map
        let mut headers = HashMap::new();
        if let Some(auth) = auth_header {
            headers.insert("Authorization".to_string(), auth);
        }

        // Get resource attributes
        let resource_attrs_str = spin_sdk::variables::get("otel_resource_attributes")
            .unwrap_or_else(|_| "service.name=wasmcp-otel-exporter,deployment.environment=production".to_string());
        println!("[OTEL] Resource attributes: {}", resource_attrs_str);

        OtlpConfig {
            endpoint,
            headers,
            service_name,
            resource_attributes: parse_resource_attributes(&resource_attrs_str),
        }
    }

    /// Convert our span data to OTLP protobuf format and send to Grafana
    fn export_span(span: &SpanImpl) {
        println!("[OTEL] Starting export for span: {}", span.name);

        let config = Self::get_config();
        println!("[OTEL] Using endpoint: {}", config.endpoint);
        println!("[OTEL] Service name: {}", config.service_name);

        let proto_data = Self::create_otlp_proto(span, &config);
        println!("[OTEL] Created protobuf data with {} resource spans", proto_data.resource_spans.len());

        // Serialize to protobuf binary format
        use prost::Message;
        let proto_bytes = proto_data.encode_to_vec();
        println!("[OTEL] Serialized to {} bytes", proto_bytes.len());

        Self::send_to_grafana(&config, &proto_bytes);
    }

    fn create_otlp_proto(span: &SpanImpl, config: &OtlpConfig) -> otlp_trace::TracesData {
        println!("[OTEL] Converting span '{}' to protobuf format", span.name);

        // Convert span context for protobuf
        let trace_id = hex::decode(&span.context.trace_id).unwrap_or_default();
        let span_id = hex::decode(&span.context.span_id).unwrap_or_default();
        let parent_span_id = span.parent_context
            .as_ref()
            .and_then(|ctx| hex::decode(&ctx.span_id).ok())
            .unwrap_or_default();

        // Access RefCell fields
        let events = span.events.borrow();
        let end_time = *span.end_time_nanos.borrow();
        let status = span.status.borrow().clone();

        println!("[OTEL] Span has {} events, end_time: {:?}", events.len(), end_time);

        // Create protobuf span
        let proto_span = otlp_trace::Span {
            trace_id,
            span_id,
            parent_span_id,
            name: span.name.clone(),
            kind: span_kind_to_proto(&span.kind),
            start_time_unix_nano: span.start_time_nanos,
            end_time_unix_nano: end_time.unwrap_or(span.start_time_nanos + 1_000_000),
            flags: span.context.trace_flags.bits() as u32,
            attributes: span.attributes.iter().map(|(k, v)| {
                otlp_common::KeyValue {
                    key: k.clone(),
                    value: Some(otlp_common::AnyValue {
                        value: Some(otlp_common::any_value::Value::StringValue(v.clone())),
                    }),
                }
            }).collect(),
            events: events.iter().map(|(name, timestamp)| {
                otlp_trace::span::Event {
                    time_unix_nano: *timestamp,
                    name: name.clone(),
                    attributes: vec![],
                    dropped_attributes_count: 0,
                }
            }).collect(),
            status: Some(status_to_proto(&status)),
            links: vec![],
            dropped_attributes_count: 0,
            dropped_events_count: 0,
            dropped_links_count: 0,
            trace_state: "".to_string(),
        };

        // Create protobuf traces data
        otlp_trace::TracesData {
            resource_spans: vec![otlp_trace::ResourceSpans {
                resource: Some(otlp_resource::Resource {
                    attributes: {
                        let mut attrs = vec![
                            otlp_common::KeyValue {
                                key: "service.name".to_string(),
                                value: Some(otlp_common::AnyValue {
                                    value: Some(otlp_common::any_value::Value::StringValue(config.service_name.clone())),
                                }),
                            }
                        ];
                        // Add all resource attributes from config
                        attrs.extend(config.resource_attributes.iter().map(|(k, v)| {
                            otlp_common::KeyValue {
                                key: k.clone(),
                                value: Some(otlp_common::AnyValue {
                                    value: Some(otlp_common::any_value::Value::StringValue(v.clone())),
                                }),
                            }
                        }));
                        attrs
                    },
                    dropped_attributes_count: 0,
                }),
                scope_spans: vec![otlp_trace::ScopeSpans {
                    scope: Some(otlp_common::InstrumentationScope {
                        name: "wasmcp-otel-exporter".to_string(),
                        version: "0.1.0".to_string(),
                        attributes: vec![],
                        dropped_attributes_count: 0,
                    }),
                    spans: vec![proto_span],
                    schema_url: "".to_string(),
                }],
                schema_url: "".to_string(),
            }],
        }
    }

    fn send_to_grafana(config: &OtlpConfig, payload: &[u8]) {
        // Append the traces-specific path to the base OTLP endpoint
        let url = format!("{}/v1/traces", config.endpoint);
        println!("[OTEL] Sending {} bytes to: {}", payload.len(), url);

        let payload = payload.to_vec();
        let headers = config.headers.clone();


        // Use spin_sdk::http::run for async HTTP requests
        let result = spin_sdk::http::run(async move {
            println!("[OTEL] Building HTTP request...");

            // Create HTTP request using Spin SDK
            let mut builder = spin_sdk::http::Request::builder();
            builder.method(spin_sdk::http::Method::Post);
            builder.uri(&url);
            builder.header("content-type", "application/x-protobuf");

            // Add configured headers (like Authorization)
            for (key, value) in &headers {
                println!("[OTEL] Adding header: {} = {}", key, if key.to_lowercase().contains("auth") { "[REDACTED]" } else { value });
                builder.header(key, value);
            }

            let request = builder.body(payload).build();
            println!("[OTEL] Sending HTTP request...");

            // Send the request
            let response: spin_sdk::http::Response = spin_sdk::http::send(request)
                .await
                .map_err(|e| {
                    let error_msg = format!("Failed to send OTLP data to Grafana: {e:?}");
                    println!("[OTEL] ERROR: {}", error_msg);
                    error_msg
                })?;

            println!("[OTEL] SUCCESS: OTLP data sent successfully to Grafana");
            println!("[OTEL] Response status: {}", response.status());

            let body = response.into_body();
            if let Ok(body_str) = String::from_utf8(body) {
                if !body_str.is_empty() {
                    println!("[OTEL] Response body: {}", body_str);
                } else {
                    println!("[OTEL] Empty response body (expected for successful OTLP submission)");
                }
            }
            Ok::<(), String>(())
        });

        if let Err(e) = result {
            println!("[OTEL] CRITICAL ERROR: Failed to send OTLP data: {}", e);
        }
    }
}

#[derive(Debug)]
struct OtlpConfig {
    endpoint: String,
    headers: HashMap<String, String>,
    service_name: String,
    resource_attributes: HashMap<String, String>,
}

fn parse_resource_attributes(attrs_str: &str) -> HashMap<String, String> {
    let mut attributes = HashMap::new();
    for pair in attrs_str.split(',') {
        if let Some((key, value)) = pair.split_once('=') {
            attributes.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    attributes
}