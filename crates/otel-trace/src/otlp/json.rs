//! OTLP JSON serialization
//!
//! Handles conversion of WIT telemetry types to OTLP JSON format using serde.

use crate::bindings::exports::wasi::otel_sdk::trace;
use crate::bindings::wasi::otel_sdk::foundation;

use serde::{Serialize, Deserialize};
use base64::{Engine as _, engine::general_purpose};

/// Serialize spans to OTLP JSON format
pub fn serialize_to_json(
    spans: Vec<trace::SpanData>,
    service_resource: foundation::OtelResource,
) -> Result<Vec<u8>, String> {
    let request = build_otlp_json_request(spans, service_resource);

    serde_json::to_vec(&request)
        .map_err(|e| format!("Failed to serialize to JSON: {}", e))
}

/// Build OTLP JSON request structure
fn build_otlp_json_request(
    spans: Vec<trace::SpanData>,
    service_resource: foundation::OtelResource,
) -> OtlpExportTraceServiceRequest {
    // Group spans by instrumentation scope
    let mut scope_spans_map = std::collections::HashMap::new();

    for span in spans {
        let scope_key = format!("{}-{}-{}",
            span.instrumentation_scope.name,
            span.instrumentation_scope.version.as_ref().unwrap_or(&"".to_string()),
            span.instrumentation_scope.schema_url.as_ref().unwrap_or(&"".to_string())
        );

        scope_spans_map
            .entry(scope_key)
            .or_insert_with(|| (span.instrumentation_scope.clone(), Vec::new()))
            .1
            .push(span);
    }

    // Build scope spans
    let scope_spans: Vec<OtlpScopeSpans> = scope_spans_map
        .into_iter()
        .map(|(_, (scope, spans))| {
            OtlpScopeSpans {
                scope: Some(OtlpInstrumentationScope {
                    name: scope.name,
                    version: scope.version,
                    attributes: convert_attributes(&scope.attributes),
                    dropped_attributes_count: 0,
                }),
                spans: spans.into_iter().map(convert_span_to_otlp).collect(),
                schema_url: scope.schema_url,
            }
        })
        .collect();

    // Build resource spans
    let resource_spans = OtlpResourceSpans {
        resource: Some(OtlpResource {
            attributes: convert_attributes(&service_resource.attributes),
            dropped_attributes_count: 0,
        }),
        scope_spans,
        schema_url: service_resource.schema_url,
    };

    OtlpExportTraceServiceRequest {
        resource_spans: vec![resource_spans],
    }
}

/// Parse dropped counts from trace_state and remove the marker
fn parse_dropped_counts(trace_state: &str) -> ((u32, u32, u32), String) {
    if let Some(dropped_start) = trace_state.find("dropped=") {
        let rest = &trace_state[dropped_start + 8..]; // Skip "dropped="
        if let Some(comma_pos) = rest.find(',') {
            let dropped_part = &rest[..comma_pos];
            let remaining = &rest[comma_pos + 1..];
            if let Some((attrs, events, links)) = parse_dropped_values(dropped_part) {
                return ((attrs, events, links), remaining.to_string());
            }
        } else {
            // No comma, entire rest is dropped counts
            if let Some((attrs, events, links)) = parse_dropped_values(rest) {
                return ((attrs, events, links), String::new());
            }
        }
    }
    ((0, 0, 0), trace_state.to_string())
}

fn parse_dropped_values(s: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 3 {
        let attrs = parts[0].parse().ok()?;
        let events = parts[1].parse().ok()?;
        let links = parts[2].parse().ok()?;
        Some((attrs, events, links))
    } else {
        None
    }
}

/// Convert a span to OTLP JSON format
fn convert_span_to_otlp(span: trace::SpanData) -> OtlpSpan {
    // Parse dropped counts from trace_state
    let ((dropped_attrs, dropped_events, dropped_links), clean_trace_state) =
        parse_dropped_counts(&span.context.trace_state);

    OtlpSpan {
        trace_id: hex_encode(&span.context.trace_id),
        span_id: hex_encode(&span.context.span_id),
        trace_state: if clean_trace_state.is_empty() { None } else { Some(clean_trace_state) },
        parent_span_id: span.parent_span_id.map(|id| hex_encode(&id)),
        name: span.name,
        kind: convert_span_kind(&span.kind),
        start_time_unix_nano: span.start_time,
        end_time_unix_nano: span.end_time.unwrap_or(0),
        attributes: convert_attributes(&span.attributes),
        dropped_attributes_count: dropped_attrs,
        events: span.events.into_iter().map(convert_event).collect(),
        dropped_events_count: dropped_events,
        links: span.links.into_iter().map(convert_link).collect(),
        dropped_links_count: dropped_links,
        status: convert_status(&span.status),
        flags: if span.context.trace_flags & 0x01 != 0 { 1 } else { 0 },
    }
}

/// Convert span kind to OTLP format
fn convert_span_kind(kind: &trace::SpanKind) -> u32 {
    match kind {
        trace::SpanKind::Internal => 1,
        trace::SpanKind::Server => 2,
        trace::SpanKind::Client => 3,
        trace::SpanKind::Producer => 4,
        trace::SpanKind::Consumer => 5,
    }
}

/// Convert span status to OTLP format
fn convert_status(status: &trace::SpanStatus) -> Option<OtlpStatus> {
    match status {
        trace::SpanStatus::Unset => None,
        trace::SpanStatus::Ok => Some(OtlpStatus {
            message: None,
            code: 1, // STATUS_CODE_OK
        }),
        trace::SpanStatus::Error(msg) => Some(OtlpStatus {
            message: Some(msg.clone()),
            code: 2, // STATUS_CODE_ERROR
        }),
    }
}

/// Convert span event to OTLP format
fn convert_event(event: trace::SpanEvent) -> OtlpSpanEvent {
    OtlpSpanEvent {
        time_unix_nano: event.timestamp,
        name: event.name,
        attributes: convert_attributes(&event.attributes),
        dropped_attributes_count: 0,
    }
}

/// Convert span link to OTLP format
fn convert_link(link: trace::SpanLink) -> OtlpSpanLink {
    OtlpSpanLink {
        trace_id: hex_encode(&link.context.trace_id),
        span_id: hex_encode(&link.context.span_id),
        trace_state: Some(link.context.trace_state),
        attributes: convert_attributes(&link.attributes),
        dropped_attributes_count: 0,
    }
}

/// Convert attributes to OTLP format
fn convert_attributes(attributes: &[foundation::Attribute]) -> Vec<OtlpKeyValue> {
    attributes
        .iter()
        .map(|attr| OtlpKeyValue {
            key: attr.key.clone(),
            value: convert_attribute_value(&attr.value),
        })
        .collect()
}

/// Convert simple value to OTLP format
fn convert_simple_value(value: &foundation::SimpleValue) -> OtlpAnyValue {
    match value {
        foundation::SimpleValue::String(s) => OtlpAnyValue::StringValue(s.clone()),
        foundation::SimpleValue::Bool(b) => OtlpAnyValue::BoolValue(*b),
        foundation::SimpleValue::Int64(i) => OtlpAnyValue::IntValue(*i),
        foundation::SimpleValue::Float64(d) => OtlpAnyValue::DoubleValue(*d),
        foundation::SimpleValue::Bytes(bytes) => {
            // Convert bytes to base64 string
            OtlpAnyValue::StringValue(general_purpose::STANDARD.encode(bytes))
        }
    }
}

/// Convert attribute value to OTLP format
fn convert_attribute_value(value: &foundation::AttributeValue) -> OtlpAnyValue {
    match value {
        foundation::AttributeValue::String(s) => OtlpAnyValue::StringValue(s.clone()),
        foundation::AttributeValue::Bool(b) => OtlpAnyValue::BoolValue(*b),
        foundation::AttributeValue::Int64(i) => OtlpAnyValue::IntValue(*i),
        foundation::AttributeValue::Float64(d) => OtlpAnyValue::DoubleValue(*d),
        foundation::AttributeValue::Bytes(bytes) => {
            // Convert bytes to base64 string
            OtlpAnyValue::StringValue(general_purpose::STANDARD.encode(bytes))
        }
        foundation::AttributeValue::ArrayString(arr) => {
            OtlpAnyValue::ArrayValue(OtlpArrayValue {
                values: arr.iter().map(|s| OtlpAnyValue::StringValue(s.clone())).collect(),
            })
        }
        foundation::AttributeValue::ArrayBool(arr) => {
            OtlpAnyValue::ArrayValue(OtlpArrayValue {
                values: arr.iter().map(|b| OtlpAnyValue::BoolValue(*b)).collect(),
            })
        }
        foundation::AttributeValue::ArrayInt64(arr) => {
            OtlpAnyValue::ArrayValue(OtlpArrayValue {
                values: arr.iter().map(|i| OtlpAnyValue::IntValue(*i as i64)).collect(),
            })
        }
        foundation::AttributeValue::ArrayFloat64(arr) => {
            OtlpAnyValue::ArrayValue(OtlpArrayValue {
                values: arr.iter().map(|d| OtlpAnyValue::DoubleValue(*d)).collect(),
            })
        }
        foundation::AttributeValue::Kvlist(kvs) => {
            // Convert each KeyValue to OtlpKeyValue
            // Note: kvlist values are SimpleValue, not AttributeValue (no nesting)
            OtlpAnyValue::KvlistValue(OtlpKeyValueList {
                values: kvs.iter().map(|kv| OtlpKeyValue {
                    key: kv.key.clone(),
                    value: convert_simple_value(&kv.value),
                }).collect(),
            })
        }
    }
}

/// Convert bytes to hex string
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

// ============================================================================
// JSON structures for OTLP
// ============================================================================

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpExportTraceServiceRequest {
    resource_spans: Vec<OtlpResourceSpans>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpResourceSpans {
    resource: Option<OtlpResource>,
    scope_spans: Vec<OtlpScopeSpans>,
    schema_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpResource {
    attributes: Vec<OtlpKeyValue>,
    dropped_attributes_count: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpScopeSpans {
    scope: Option<OtlpInstrumentationScope>,
    spans: Vec<OtlpSpan>,
    schema_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpInstrumentationScope {
    name: String,
    version: Option<String>,
    attributes: Vec<OtlpKeyValue>,
    dropped_attributes_count: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpSpan {
    trace_id: String,
    span_id: String,
    trace_state: Option<String>,
    parent_span_id: Option<String>,
    name: String,
    kind: u32,
    start_time_unix_nano: u64,
    end_time_unix_nano: u64,
    attributes: Vec<OtlpKeyValue>,
    dropped_attributes_count: u32,
    events: Vec<OtlpSpanEvent>,
    dropped_events_count: u32,
    links: Vec<OtlpSpanLink>,
    dropped_links_count: u32,
    status: Option<OtlpStatus>,
    flags: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpSpanEvent {
    time_unix_nano: u64,
    name: String,
    attributes: Vec<OtlpKeyValue>,
    dropped_attributes_count: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpSpanLink {
    trace_id: String,
    span_id: String,
    trace_state: Option<String>,
    attributes: Vec<OtlpKeyValue>,
    dropped_attributes_count: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpStatus {
    message: Option<String>,
    code: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpKeyValue {
    key: String,
    value: OtlpAnyValue,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
enum OtlpAnyValue {
    StringValue(String),
    BoolValue(bool),
    IntValue(i64),
    DoubleValue(f64),
    ArrayValue(OtlpArrayValue),
    KvlistValue(OtlpKeyValueList),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpArrayValue {
    values: Vec<OtlpAnyValue>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpKeyValueList {
    values: Vec<OtlpKeyValue>,
}
