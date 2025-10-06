use crate::bindings::exports::wasi::otel_sdk::trace;
use crate::bindings::wasi::otel_sdk::foundation;
use crate::bindings::wasi::otel_sdk::otel_export;

use serde::{Serialize, Deserialize};
use serde_json;
use base64::{Engine as _, engine::general_purpose};

/// Serialize spans to OTLP format based on the export protocol
pub fn serialize_spans_to_otlp(
    spans: Vec<trace::SpanData>,
    service_resource: foundation::OtelResource,
    protocol: otel_export::ExportProtocol,
) -> Result<Vec<u8>, String> {
    match protocol {
        otel_export::ExportProtocol::HttpJson => serialize_to_json(spans, service_resource),
        otel_export::ExportProtocol::HttpProtobuf | otel_export::ExportProtocol::Grpc => {
            serialize_to_protobuf(spans, service_resource)
        }
    }
}

/// Serialize spans to OTLP JSON format
fn serialize_to_json(
    spans: Vec<trace::SpanData>,
    service_resource: foundation::OtelResource,
) -> Result<Vec<u8>, String> {
    let request = build_otlp_json_request(spans, service_resource);

    serde_json::to_vec(&request)
        .map_err(|e| format!("Failed to serialize to JSON: {}", e))
}

/// Serialize spans to OTLP protobuf format
fn serialize_to_protobuf(
    spans: Vec<trace::SpanData>,
    service_resource: foundation::OtelResource,
) -> Result<Vec<u8>, String> {
    // Build the protobuf message structure
    let mut buffer = Vec::new();

    // Write protobuf message header (simplified implementation)
    // Field 1: resource_spans (repeated message)
    write_protobuf_tag(&mut buffer, 1, 2); // field 1, wire type 2 (length-delimited)

    // Write resource spans
    let resource_spans = build_resource_spans_proto(spans, service_resource);
    write_protobuf_length_delimited(&mut buffer, &resource_spans);

    Ok(buffer)
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

/// Convert a span to OTLP JSON format
fn convert_span_to_otlp(span: trace::SpanData) -> OtlpSpan {
    OtlpSpan {
        trace_id: hex_encode(&span.context.trace_id),
        span_id: hex_encode(&span.context.span_id),
        trace_state: Some(span.context.trace_state),
        parent_span_id: span.parent_span_id.map(|id| hex_encode(&id)),
        name: span.name,
        kind: convert_span_kind(&span.kind),
        start_time_unix_nano: span.start_time,
        end_time_unix_nano: span.end_time.unwrap_or(0),
        attributes: convert_attributes(&span.attributes),
        dropped_attributes_count: 0,
        events: span.events.into_iter().map(convert_event).collect(),
        dropped_events_count: 0,
        links: span.links.into_iter().map(convert_link).collect(),
        dropped_links_count: 0,
        status: convert_status(&span.status),
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

/// Build resource spans protobuf message
fn build_resource_spans_proto(
    spans: Vec<trace::SpanData>,
    service_resource: foundation::OtelResource,
) -> Vec<u8> {
    let mut buffer = Vec::new();

    // Field 1: resource (message)
    if !service_resource.attributes.is_empty() {
        write_protobuf_tag(&mut buffer, 1, 2);
        let resource_proto = encode_resource_proto(&service_resource);
        write_protobuf_length_delimited(&mut buffer, &resource_proto);
    }

    // Field 2: scope_spans (repeated message)
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

    // Write each scope spans
    for (_, (scope, spans)) in scope_spans_map {
        write_protobuf_tag(&mut buffer, 2, 2);
        let scope_spans_proto = encode_scope_spans_proto(scope, spans);
        write_protobuf_length_delimited(&mut buffer, &scope_spans_proto);
    }

    // Field 3: schema_url (string) - if present
    if let Some(url) = &service_resource.schema_url {
        write_protobuf_tag(&mut buffer, 3, 2);
        write_protobuf_string(&mut buffer, url);
    }

    buffer
}

/// Encode resource to protobuf
fn encode_resource_proto(resource: &foundation::OtelResource) -> Vec<u8> {
    let mut buffer = Vec::new();

    // Field 1: attributes (repeated KeyValue)
    for attr in &resource.attributes {
        write_protobuf_tag(&mut buffer, 1, 2);
        let kv_proto = encode_key_value_proto(&attr.key, &attr.value);
        write_protobuf_length_delimited(&mut buffer, &kv_proto);
    }

    buffer
}

/// Encode scope spans to protobuf
fn encode_scope_spans_proto(scope: foundation::InstrumentationScope, spans: Vec<trace::SpanData>) -> Vec<u8> {
    let mut buffer = Vec::new();

    // Field 1: scope (InstrumentationScope message)
    write_protobuf_tag(&mut buffer, 1, 2);
    let scope_proto = encode_instrumentation_scope_proto(&scope);
    write_protobuf_length_delimited(&mut buffer, &scope_proto);

    // Field 2: spans (repeated Span)
    for span in spans {
        write_protobuf_tag(&mut buffer, 2, 2);
        let span_proto = encode_span_proto(span);
        write_protobuf_length_delimited(&mut buffer, &span_proto);
    }

    // Field 3: schema_url (string) - if present
    if let Some(url) = &scope.schema_url {
        write_protobuf_tag(&mut buffer, 3, 2);
        write_protobuf_string(&mut buffer, url);
    }

    buffer
}

/// Encode instrumentation scope to protobuf
fn encode_instrumentation_scope_proto(scope: &foundation::InstrumentationScope) -> Vec<u8> {
    let mut buffer = Vec::new();

    // Field 1: name (string)
    write_protobuf_tag(&mut buffer, 1, 2);
    write_protobuf_string(&mut buffer, &scope.name);

    // Field 2: version (string) - if present
    if let Some(version) = &scope.version {
        write_protobuf_tag(&mut buffer, 2, 2);
        write_protobuf_string(&mut buffer, version);
    }

    // Field 3: attributes (repeated KeyValue)
    for attr in &scope.attributes {
        write_protobuf_tag(&mut buffer, 3, 2);
        let kv_proto = encode_key_value_proto(&attr.key, &attr.value);
        write_protobuf_length_delimited(&mut buffer, &kv_proto);
    }

    buffer
}

/// Encode span to protobuf
fn encode_span_proto(span: trace::SpanData) -> Vec<u8> {
    let mut buffer = Vec::new();

    // Field 1: trace_id (bytes)
    write_protobuf_tag(&mut buffer, 1, 2);
    write_protobuf_bytes(&mut buffer, &span.context.trace_id);

    // Field 2: span_id (bytes)
    write_protobuf_tag(&mut buffer, 2, 2);
    write_protobuf_bytes(&mut buffer, &span.context.span_id);

    // Field 3: trace_state (string) - if not empty
    if !span.context.trace_state.is_empty() {
        write_protobuf_tag(&mut buffer, 3, 2);
        write_protobuf_string(&mut buffer, &span.context.trace_state);
    }

    // Field 4: parent_span_id (bytes) - if present
    if let Some(parent_id) = &span.parent_span_id {
        write_protobuf_tag(&mut buffer, 4, 2);
        write_protobuf_bytes(&mut buffer, parent_id);
    }

    // Field 5: name (string)
    write_protobuf_tag(&mut buffer, 5, 2);
    write_protobuf_string(&mut buffer, &span.name);

    // Field 6: kind (enum as int32)
    write_protobuf_tag(&mut buffer, 6, 0); // varint
    write_protobuf_varint(&mut buffer, convert_span_kind(&span.kind) as u64);

    // Field 7: start_time_unix_nano (fixed64)
    write_protobuf_tag(&mut buffer, 7, 1); // fixed64
    write_protobuf_fixed64(&mut buffer, span.start_time);

    // Field 8: end_time_unix_nano (fixed64) - if present
    if let Some(end_time) = span.end_time {
        write_protobuf_tag(&mut buffer, 8, 1); // fixed64
        write_protobuf_fixed64(&mut buffer, end_time);
    }

    // Field 9: attributes (repeated KeyValue)
    for attr in &span.attributes {
        write_protobuf_tag(&mut buffer, 9, 2);
        let kv_proto = encode_key_value_proto(&attr.key, &attr.value);
        write_protobuf_length_delimited(&mut buffer, &kv_proto);
    }

    // Field 11: events (repeated Event)
    for event in &span.events {
        write_protobuf_tag(&mut buffer, 11, 2);
        let event_proto = encode_event_proto(event);
        write_protobuf_length_delimited(&mut buffer, &event_proto);
    }

    // Field 13: links (repeated Link)
    for link in &span.links {
        write_protobuf_tag(&mut buffer, 13, 2);
        let link_proto = encode_link_proto(link);
        write_protobuf_length_delimited(&mut buffer, &link_proto);
    }

    // Field 15: status (Status message) - if not unset
    match &span.status {
        trace::SpanStatus::Unset => {}
        status => {
            write_protobuf_tag(&mut buffer, 15, 2);
            let status_proto = encode_status_proto(status);
            write_protobuf_length_delimited(&mut buffer, &status_proto);
        }
    }

    buffer
}

/// Encode span event to protobuf
fn encode_event_proto(event: &trace::SpanEvent) -> Vec<u8> {
    let mut buffer = Vec::new();

    // Field 1: time_unix_nano (fixed64)
    write_protobuf_tag(&mut buffer, 1, 1);
    write_protobuf_fixed64(&mut buffer, event.timestamp);

    // Field 2: name (string)
    write_protobuf_tag(&mut buffer, 2, 2);
    write_protobuf_string(&mut buffer, &event.name);

    // Field 3: attributes (repeated KeyValue)
    for attr in &event.attributes {
        write_protobuf_tag(&mut buffer, 3, 2);
        let kv_proto = encode_key_value_proto(&attr.key, &attr.value);
        write_protobuf_length_delimited(&mut buffer, &kv_proto);
    }

    buffer
}

/// Encode span link to protobuf
fn encode_link_proto(link: &trace::SpanLink) -> Vec<u8> {
    let mut buffer = Vec::new();

    // Field 1: trace_id (bytes)
    write_protobuf_tag(&mut buffer, 1, 2);
    write_protobuf_bytes(&mut buffer, &link.context.trace_id);

    // Field 2: span_id (bytes)
    write_protobuf_tag(&mut buffer, 2, 2);
    write_protobuf_bytes(&mut buffer, &link.context.span_id);

    // Field 3: trace_state (string) - if not empty
    if !link.context.trace_state.is_empty() {
        write_protobuf_tag(&mut buffer, 3, 2);
        write_protobuf_string(&mut buffer, &link.context.trace_state);
    }

    // Field 4: attributes (repeated KeyValue)
    for attr in &link.attributes {
        write_protobuf_tag(&mut buffer, 4, 2);
        let kv_proto = encode_key_value_proto(&attr.key, &attr.value);
        write_protobuf_length_delimited(&mut buffer, &kv_proto);
    }

    buffer
}

/// Encode status to protobuf
fn encode_status_proto(status: &trace::SpanStatus) -> Vec<u8> {
    let mut buffer = Vec::new();

    match status {
        trace::SpanStatus::Ok => {
            // Field 2: code (enum as int32)
            write_protobuf_tag(&mut buffer, 2, 0);
            write_protobuf_varint(&mut buffer, 1); // STATUS_CODE_OK
        }
        trace::SpanStatus::Error(msg) => {
            // Field 1: message (string)
            write_protobuf_tag(&mut buffer, 1, 2);
            write_protobuf_string(&mut buffer, msg);

            // Field 2: code (enum as int32)
            write_protobuf_tag(&mut buffer, 2, 0);
            write_protobuf_varint(&mut buffer, 2); // STATUS_CODE_ERROR
        }
        trace::SpanStatus::Unset => unreachable!("Unset status should not be encoded"),
    }

    buffer
}

/// Encode simple value to protobuf (for kvlist)
fn encode_simple_value_proto(value: &foundation::SimpleValue) -> Vec<u8> {
    let mut buffer = Vec::new();

    match value {
        foundation::SimpleValue::String(s) => {
            // Field 1: string_value (string)
            write_protobuf_tag(&mut buffer, 1, 2);
            write_protobuf_string(&mut buffer, s);
        }
        foundation::SimpleValue::Bool(b) => {
            // Field 2: bool_value (bool)
            write_protobuf_tag(&mut buffer, 2, 0);
            write_protobuf_varint(&mut buffer, if *b { 1 } else { 0 });
        }
        foundation::SimpleValue::Int64(i) => {
            // Field 3: int_value (int64)
            write_protobuf_tag(&mut buffer, 3, 0);
            write_protobuf_varint(&mut buffer, *i as u64);
        }
        foundation::SimpleValue::Float64(d) => {
            // Field 4: double_value (double)
            write_protobuf_tag(&mut buffer, 4, 1);
            write_protobuf_double(&mut buffer, *d);
        }
        foundation::SimpleValue::Bytes(bytes) => {
            // Field 7: bytes_value (bytes)
            write_protobuf_tag(&mut buffer, 7, 2);
            write_protobuf_bytes(&mut buffer, bytes);
        }
    }

    buffer
}

/// Encode key-value pair to protobuf (for attributes - AttributeValue)
fn encode_key_value_proto(key: &str, value: &foundation::AttributeValue) -> Vec<u8> {
    let mut buffer = Vec::new();

    // Field 1: key (string)
    write_protobuf_tag(&mut buffer, 1, 2);
    write_protobuf_string(&mut buffer, key);

    // Field 2: value (AnyValue message)
    write_protobuf_tag(&mut buffer, 2, 2);
    let value_proto = encode_any_value_proto(value);
    write_protobuf_length_delimited(&mut buffer, &value_proto);

    buffer
}

/// Encode key-value pair to protobuf (for kvlist - SimpleValue)
fn encode_key_simple_value_proto(key: &str, value: &foundation::SimpleValue) -> Vec<u8> {
    let mut buffer = Vec::new();

    // Field 1: key (string)
    write_protobuf_tag(&mut buffer, 1, 2);
    write_protobuf_string(&mut buffer, key);

    // Field 2: value (AnyValue message with SimpleValue)
    write_protobuf_tag(&mut buffer, 2, 2);
    let value_proto = encode_simple_value_proto(value);
    write_protobuf_length_delimited(&mut buffer, &value_proto);

    buffer
}

/// Encode any value to protobuf
fn encode_any_value_proto(value: &foundation::AttributeValue) -> Vec<u8> {
    let mut buffer = Vec::new();

    match value {
        foundation::AttributeValue::String(s) => {
            // Field 1: string_value (string)
            write_protobuf_tag(&mut buffer, 1, 2);
            write_protobuf_string(&mut buffer, s);
        }
        foundation::AttributeValue::Bool(b) => {
            // Field 2: bool_value (bool)
            write_protobuf_tag(&mut buffer, 2, 0);
            write_protobuf_varint(&mut buffer, if *b { 1 } else { 0 });
        }
        foundation::AttributeValue::Int64(i) => {
            // Field 3: int_value (int64)
            write_protobuf_tag(&mut buffer, 3, 0);
            write_protobuf_varint(&mut buffer, *i as u64);
        }
        foundation::AttributeValue::Float64(d) => {
            // Field 4: double_value (double)
            write_protobuf_tag(&mut buffer, 4, 1);
            write_protobuf_double(&mut buffer, *d);
        }
        foundation::AttributeValue::Bytes(bytes) => {
            // Field 7: bytes_value (bytes)
            write_protobuf_tag(&mut buffer, 7, 2);
            write_protobuf_bytes(&mut buffer, bytes);
        }
        foundation::AttributeValue::ArrayString(arr) => {
            // Field 5: array_value (ArrayValue message)
            write_protobuf_tag(&mut buffer, 5, 2);
            let array_proto = encode_array_value_proto_strings(arr);
            write_protobuf_length_delimited(&mut buffer, &array_proto);
        }
        foundation::AttributeValue::ArrayBool(arr) => {
            // Field 5: array_value (ArrayValue message)
            write_protobuf_tag(&mut buffer, 5, 2);
            let array_proto = encode_array_value_proto_bools(arr);
            write_protobuf_length_delimited(&mut buffer, &array_proto);
        }
        foundation::AttributeValue::ArrayInt64(arr) => {
            // Field 5: array_value (ArrayValue message)
            write_protobuf_tag(&mut buffer, 5, 2);
            let array_proto = encode_array_value_proto_ints(arr);
            write_protobuf_length_delimited(&mut buffer, &array_proto);
        }
        foundation::AttributeValue::ArrayFloat64(arr) => {
            // Field 5: array_value (ArrayValue message)
            write_protobuf_tag(&mut buffer, 5, 2);
            let array_proto = encode_array_value_proto_doubles(arr);
            write_protobuf_length_delimited(&mut buffer, &array_proto);
        }
        foundation::AttributeValue::Kvlist(kvs) => {
            // Field 6: kvlist_value (KeyValueList message)
            write_protobuf_tag(&mut buffer, 6, 2);
            let kvlist_proto = encode_kvlist_value_proto(kvs);
            write_protobuf_length_delimited(&mut buffer, &kvlist_proto);
        }
    }

    buffer
}

/// Encode array value to protobuf (strings)
fn encode_array_value_proto_strings(values: &[String]) -> Vec<u8> {
    let mut buffer = Vec::new();

    // Field 1: values (repeated AnyValue)
    for value in values {
        write_protobuf_tag(&mut buffer, 1, 2);
        let value_proto = encode_any_value_proto(&foundation::AttributeValue::String(value.clone()));
        write_protobuf_length_delimited(&mut buffer, &value_proto);
    }

    buffer
}

/// Encode array value to protobuf (bools)
fn encode_array_value_proto_bools(values: &[bool]) -> Vec<u8> {
    let mut buffer = Vec::new();

    // Field 1: values (repeated AnyValue)
    for value in values {
        write_protobuf_tag(&mut buffer, 1, 2);
        let value_proto = encode_any_value_proto(&foundation::AttributeValue::Bool(*value));
        write_protobuf_length_delimited(&mut buffer, &value_proto);
    }

    buffer
}

/// Encode array value to protobuf (ints)
fn encode_array_value_proto_ints(values: &[i64]) -> Vec<u8> {
    let mut buffer = Vec::new();

    // Field 1: values (repeated AnyValue)
    for value in values {
        write_protobuf_tag(&mut buffer, 1, 2);
        let value_proto = encode_any_value_proto(&foundation::AttributeValue::Int64(*value));
        write_protobuf_length_delimited(&mut buffer, &value_proto);
    }

    buffer
}

/// Encode array value to protobuf (doubles)
fn encode_array_value_proto_doubles(values: &[f64]) -> Vec<u8> {
    let mut buffer = Vec::new();

    // Field 1: values (repeated AnyValue)
    for value in values {
        write_protobuf_tag(&mut buffer, 1, 2);
        let value_proto = encode_any_value_proto(&foundation::AttributeValue::Float64(*value));
        write_protobuf_length_delimited(&mut buffer, &value_proto);
    }

    buffer
}

/// Encode key-value list to protobuf
fn encode_kvlist_value_proto(kvs: &[foundation::KeyValue]) -> Vec<u8> {
    let mut buffer = Vec::new();

    // Field 1: values (repeated KeyValue with SimpleValue)
    for kv in kvs {
        write_protobuf_tag(&mut buffer, 1, 2);
        let kv_proto = encode_key_simple_value_proto(&kv.key, &kv.value);
        write_protobuf_length_delimited(&mut buffer, &kv_proto);
    }

    buffer
}

// ============================================================================
// Protobuf encoding helpers
// ============================================================================

/// Write a protobuf tag (field number and wire type)
fn write_protobuf_tag(buffer: &mut Vec<u8>, field_number: u32, wire_type: u8) {
    let tag = (field_number << 3) | (wire_type as u32);
    write_protobuf_varint(buffer, tag as u64);
}

/// Write a protobuf varint
fn write_protobuf_varint(buffer: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        buffer.push(((value & 0x7F) | 0x80) as u8);
        value >>= 7;
    }
    buffer.push(value as u8);
}

/// Write a protobuf length-delimited field
fn write_protobuf_length_delimited(buffer: &mut Vec<u8>, data: &[u8]) {
    write_protobuf_varint(buffer, data.len() as u64);
    buffer.extend_from_slice(data);
}

/// Write a protobuf string
fn write_protobuf_string(buffer: &mut Vec<u8>, s: &str) {
    write_protobuf_length_delimited(buffer, s.as_bytes());
}

/// Write protobuf bytes
fn write_protobuf_bytes(buffer: &mut Vec<u8>, bytes: &[u8]) {
    write_protobuf_length_delimited(buffer, bytes);
}

/// Write a protobuf fixed64
fn write_protobuf_fixed64(buffer: &mut Vec<u8>, value: u64) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

/// Write a protobuf double
fn write_protobuf_double(buffer: &mut Vec<u8>, value: f64) {
    buffer.extend_from_slice(&value.to_le_bytes());
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