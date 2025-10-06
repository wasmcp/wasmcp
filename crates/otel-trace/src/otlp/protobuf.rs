//! OTLP Protobuf serialization using prost
//!
//! Converts WIT telemetry types to prost-generated OTLP types and serializes to protobuf.
//! Uses official OpenTelemetry proto definitions for correctness.

use crate::bindings::exports::wasi::otel_sdk::trace;
use crate::bindings::wasi::otel_sdk::foundation;
use crate::otlp::proto;

use prost::Message;

/// Serialize spans to OTLP protobuf format using prost
pub fn serialize_to_protobuf(
    spans: Vec<trace::SpanData>,
    service_resource: foundation::OtelResource,
) -> Result<Vec<u8>, String> {
    // Convert WIT types to prost-generated types
    let request = convert_to_export_request(spans, service_resource)?;

    // Use prost's auto-generated encoding
    let mut buffer = Vec::new();
    request.encode(&mut buffer)
        .map_err(|e| format!("Protobuf encoding failed: {}", e))?;

    Ok(buffer)
}

/// Convert WIT spans to OTLP ExportTraceServiceRequest
fn convert_to_export_request(
    spans: Vec<trace::SpanData>,
    service_resource: foundation::OtelResource,
) -> Result<proto::collector::trace::v1::ExportTraceServiceRequest, String> {
    // Group spans by instrumentation scope
    let scope_spans = group_and_convert_spans(spans);

    let resource_spans = proto::trace::v1::ResourceSpans {
        resource: Some(convert_resource(&service_resource)),
        scope_spans,
        schema_url: service_resource.schema_url.unwrap_or_default(),
    };

    Ok(proto::collector::trace::v1::ExportTraceServiceRequest {
        resource_spans: vec![resource_spans],
    })
}

/// Group spans by scope and convert to prost types
fn group_and_convert_spans(spans: Vec<trace::SpanData>) -> Vec<proto::trace::v1::ScopeSpans> {
    let mut scope_map: std::collections::HashMap<String, (foundation::InstrumentationScope, Vec<trace::SpanData>)>
        = std::collections::HashMap::new();

    // Group spans by instrumentation scope
    for span in spans {
        let scope_key = format!(
            "{}-{}-{}",
            span.instrumentation_scope.name,
            span.instrumentation_scope.version.as_ref().unwrap_or(&"".to_string()),
            span.instrumentation_scope.schema_url.as_ref().unwrap_or(&"".to_string())
        );

        scope_map
            .entry(scope_key)
            .or_insert_with(|| (span.instrumentation_scope.clone(), Vec::new()))
            .1
            .push(span);
    }

    // Convert each scope group to ScopeSpans
    scope_map
        .into_iter()
        .map(|(_, (scope, spans))| {
            proto::trace::v1::ScopeSpans {
                scope: Some(convert_instrumentation_scope(&scope)),
                spans: spans.into_iter().map(convert_span).collect(),
                schema_url: scope.schema_url.unwrap_or_default(),
            }
        })
        .collect()
}

/// Convert WIT Resource to prost Resource
fn convert_resource(resource: &foundation::OtelResource) -> proto::resource::v1::Resource {
    proto::resource::v1::Resource {
        attributes: convert_attributes(&resource.attributes),
        dropped_attributes_count: 0,
        entity_refs: vec![], // Entity references (optional, not currently used)
    }
}

/// Convert WIT InstrumentationScope to prost InstrumentationScope
fn convert_instrumentation_scope(scope: &foundation::InstrumentationScope) -> proto::common::v1::InstrumentationScope {
    proto::common::v1::InstrumentationScope {
        name: scope.name.clone(),
        version: scope.version.clone().unwrap_or_default(),
        attributes: convert_attributes(&scope.attributes),
        dropped_attributes_count: 0,
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

/// Convert WIT SpanData to prost Span
fn convert_span(span: trace::SpanData) -> proto::trace::v1::Span {
    // Parse dropped counts from trace_state
    let ((dropped_attrs, dropped_events, dropped_links), clean_trace_state) =
        parse_dropped_counts(&span.context.trace_state);

    proto::trace::v1::Span {
        trace_id: span.context.trace_id,
        span_id: span.context.span_id,
        trace_state: clean_trace_state,
        parent_span_id: span.parent_span_id.unwrap_or_default(),
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
        status: Some(convert_status(&span.status)),
        flags: if span.context.trace_flags & 0x01 != 0 { 1 } else { 0 }, // Sampled flag
    }
}

/// Convert WIT SpanKind to prost SpanKind
fn convert_span_kind(kind: &trace::SpanKind) -> i32 {
    match kind {
        trace::SpanKind::Internal => proto::trace::v1::span::SpanKind::Internal as i32,
        trace::SpanKind::Server => proto::trace::v1::span::SpanKind::Server as i32,
        trace::SpanKind::Client => proto::trace::v1::span::SpanKind::Client as i32,
        trace::SpanKind::Producer => proto::trace::v1::span::SpanKind::Producer as i32,
        trace::SpanKind::Consumer => proto::trace::v1::span::SpanKind::Consumer as i32,
    }
}

/// Convert WIT SpanStatus to prost Status
fn convert_status(status: &trace::SpanStatus) -> proto::trace::v1::Status {
    match status {
        trace::SpanStatus::Unset => proto::trace::v1::Status {
            message: String::new(),
            code: proto::trace::v1::status::StatusCode::Unset as i32,
        },
        trace::SpanStatus::Ok => proto::trace::v1::Status {
            message: String::new(),
            code: proto::trace::v1::status::StatusCode::Ok as i32,
        },
        trace::SpanStatus::Error(msg) => proto::trace::v1::Status {
            message: msg.clone(),
            code: proto::trace::v1::status::StatusCode::Error as i32,
        },
    }
}

/// Convert WIT SpanEvent to prost Event
fn convert_event(event: trace::SpanEvent) -> proto::trace::v1::span::Event {
    proto::trace::v1::span::Event {
        time_unix_nano: event.timestamp,
        name: event.name,
        attributes: convert_attributes(&event.attributes),
        dropped_attributes_count: 0,
    }
}

/// Convert WIT SpanLink to prost Link
fn convert_link(link: trace::SpanLink) -> proto::trace::v1::span::Link {
    proto::trace::v1::span::Link {
        trace_id: link.context.trace_id,
        span_id: link.context.span_id,
        trace_state: link.context.trace_state,
        attributes: convert_attributes(&link.attributes),
        dropped_attributes_count: 0,
        flags: if link.context.trace_flags & 0x01 != 0 { 1 } else { 0 },
    }
}

/// Convert WIT Attributes to prost KeyValue list
fn convert_attributes(attributes: &[foundation::Attribute]) -> Vec<proto::common::v1::KeyValue> {
    attributes
        .iter()
        .map(|attr| proto::common::v1::KeyValue {
            key: attr.key.clone(),
            value: Some(convert_attribute_value(&attr.value)),
        })
        .collect()
}

/// Convert WIT AttributeValue to prost AnyValue
fn convert_attribute_value(value: &foundation::AttributeValue) -> proto::common::v1::AnyValue {
    use proto::common::v1::any_value::Value;

    let value_variant = match value {
        foundation::AttributeValue::String(s) => Value::StringValue(s.clone()),
        foundation::AttributeValue::Bool(b) => Value::BoolValue(*b),
        foundation::AttributeValue::Int64(i) => Value::IntValue(*i),
        foundation::AttributeValue::Float64(f) => Value::DoubleValue(*f),
        foundation::AttributeValue::Bytes(bytes) => Value::BytesValue(bytes.clone()),
        foundation::AttributeValue::ArrayString(arr) => {
            Value::ArrayValue(proto::common::v1::ArrayValue {
                values: arr
                    .iter()
                    .map(|s| proto::common::v1::AnyValue {
                        value: Some(Value::StringValue(s.clone())),
                    })
                    .collect(),
            })
        }
        foundation::AttributeValue::ArrayBool(arr) => {
            Value::ArrayValue(proto::common::v1::ArrayValue {
                values: arr
                    .iter()
                    .map(|b| proto::common::v1::AnyValue {
                        value: Some(Value::BoolValue(*b)),
                    })
                    .collect(),
            })
        }
        foundation::AttributeValue::ArrayInt64(arr) => {
            Value::ArrayValue(proto::common::v1::ArrayValue {
                values: arr
                    .iter()
                    .map(|i| proto::common::v1::AnyValue {
                        value: Some(Value::IntValue(*i)),
                    })
                    .collect(),
            })
        }
        foundation::AttributeValue::ArrayFloat64(arr) => {
            Value::ArrayValue(proto::common::v1::ArrayValue {
                values: arr
                    .iter()
                    .map(|f| proto::common::v1::AnyValue {
                        value: Some(Value::DoubleValue(*f)),
                    })
                    .collect(),
            })
        }
        foundation::AttributeValue::Kvlist(kvs) => {
            Value::KvlistValue(proto::common::v1::KeyValueList {
                values: kvs
                    .iter()
                    .map(|kv| proto::common::v1::KeyValue {
                        key: kv.key.clone(),
                        value: Some(convert_simple_value(&kv.value)),
                    })
                    .collect(),
            })
        }
    };

    proto::common::v1::AnyValue {
        value: Some(value_variant),
    }
}

/// Convert WIT SimpleValue to prost AnyValue
fn convert_simple_value(value: &foundation::SimpleValue) -> proto::common::v1::AnyValue {
    use proto::common::v1::any_value::Value;

    let value_variant = match value {
        foundation::SimpleValue::String(s) => Value::StringValue(s.clone()),
        foundation::SimpleValue::Bool(b) => Value::BoolValue(*b),
        foundation::SimpleValue::Int64(i) => Value::IntValue(*i),
        foundation::SimpleValue::Float64(f) => Value::DoubleValue(*f),
        foundation::SimpleValue::Bytes(bytes) => Value::BytesValue(bytes.clone()),
    };

    proto::common::v1::AnyValue {
        value: Some(value_variant),
    }
}
