//! OTLP Protobuf serialization using prost.
//!
//! Converts WIT telemetry types to prost-generated OTLP types and serializes to protobuf.
//! Uses official OpenTelemetry proto definitions from the `opentelemetry-proto` crate.
//!
//! # Protobuf Encoding
//!
//! This module uses Google's Protocol Buffers (protobuf) for efficient binary serialization:
//! - ~30-50% smaller than JSON for typical trace data
//! - Faster encoding/decoding than JSON
//! - Official OTLP wire format for gRPC and HTTP/protobuf
//!
//! # Type Safety
//!
//! All protobuf types are generated from official `.proto` files via the
//! `opentelemetry-proto` crate, ensuring compatibility with OTLP collectors:
//! - Jaeger
//! - Grafana Cloud
//! - OpenTelemetry Collector
//! - Datadog
//!
//! # Key Features
//!
//! - **Binary IDs**: Trace and span IDs are raw bytes (not hex-encoded)
//! - **Dropped Counts**: Decodes from trace_state and encodes to OTLP fields (Issue #4)
//! - **Compact Encoding**: Varint encoding for integers, no field names in output
//! - **Status Codes**: Maps WIT SpanStatus to OTLP status codes (0=UNSET, 1=OK, 2=ERROR)

use crate::bindings::exports::wasi::otel_sdk::trace;
use crate::bindings::wasi::otel_sdk::foundation;
use crate::error::SerializationError;
use crate::otlp::proto;

use prost::Message;

/// Serializes spans to OTLP protobuf format.
///
/// Converts WIT span data into OTLP-compliant protobuf bytes ready for HTTP or gRPC transport.
/// Uses the official `opentelemetry-proto` crate for type-safe protobuf encoding.
///
/// # Arguments
///
/// * `spans` - Vector of span data to serialize
/// * `service_resource` - Service resource attributes (appears in ResourceSpans)
///
/// # Returns
///
/// - `Ok(Vec<u8>)` - Binary protobuf-encoded bytes
/// - `Err(SerializationError::ProtobufEncoding)` - prost encoding failed
///
/// # Size Characteristics
///
/// Protobuf is significantly more compact than JSON:
/// - Single span: ~100-200 bytes (vs. 300-500 bytes JSON)
/// - 100 spans: ~10-20 KB (vs. 30-50 KB JSON)
///
/// # Protocol Compatibility
///
/// This encoding is compatible with:
/// - OTLP/HTTP with `Content-Type: application/x-protobuf`
/// - OTLP/gRPC (identical binary format)
///
/// # Examples
///
/// ```no_run
/// # use otel_trace::otlp::protobuf::serialize_to_protobuf;
/// # use otel_trace::bindings::exports::wasi::otel_sdk::trace::SpanData;
/// # use otel_trace::bindings::wasi::otel_sdk::foundation::OtelResource;
/// # fn example(spans: Vec<SpanData>, resource: OtelResource) -> Result<(), Box<dyn std::error::Error>> {
/// let proto_bytes = serialize_to_protobuf(spans, resource)?;
///
/// // Send via HTTP
/// // POST /v1/traces
/// // Content-Type: application/x-protobuf
/// // Body: proto_bytes
/// # Ok(())
/// # }
/// ```
pub fn serialize_to_protobuf(
    spans: Vec<trace::SpanData>,
    service_resource: foundation::OtelResource,
) -> Result<Vec<u8>, SerializationError> {
    // Convert WIT types to prost-generated types
    let request = convert_to_export_request(spans, service_resource)?;

    // Use prost's auto-generated encoding
    let mut buffer = Vec::new();
    request.encode(&mut buffer)
        .map_err(SerializationError::from)?;

    Ok(buffer)
}

/// Convert WIT spans to OTLP ExportTraceServiceRequest
fn convert_to_export_request(
    spans: Vec<trace::SpanData>,
    service_resource: foundation::OtelResource,
) -> Result<proto::collector::trace::v1::ExportTraceServiceRequest, SerializationError> {
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

/// Convert WIT SpanData to prost Span
fn convert_span(span: trace::SpanData) -> proto::trace::v1::Span {
    proto::trace::v1::Span {
        trace_id: span.context.trace_id,
        span_id: span.context.span_id,
        trace_state: span.context.trace_state,
        parent_span_id: span.parent_span_id.unwrap_or_default(),
        name: span.name,
        kind: convert_span_kind(&span.kind),
        start_time_unix_nano: span.start_time,
        end_time_unix_nano: span.end_time.unwrap_or(0),
        attributes: convert_attributes(&span.attributes),
        dropped_attributes_count: span.dropped_attributes_count,
        events: span.events.into_iter().map(convert_event).collect(),
        dropped_events_count: span.dropped_events_count,
        links: span.links.into_iter().map(convert_link).collect(),
        dropped_links_count: span.dropped_links_count,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::wasi::otel_sdk::context::SpanContext;

    fn create_test_span() -> trace::SpanData {
        trace::SpanData {
            name: "test-span".to_string(),
            context: SpanContext {
                trace_id: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
                span_id: vec![1, 2, 3, 4, 5, 6, 7, 8],
                trace_flags: 1, // Sampled
                trace_state: String::new(),
                is_remote: false,
            },
            parent_span_id: None,
            kind: trace::SpanKind::Internal,
            start_time: 1000000,
            end_time: Some(2000000),
            attributes: vec![
                foundation::Attribute {
                    key: "test.key".to_string(),
                    value: foundation::AttributeValue::String("test-value".to_string()),
                },
            ],
            events: vec![],
            links: vec![],
            status: trace::SpanStatus::Ok,
            instrumentation_scope: foundation::InstrumentationScope {
                name: "test-scope".to_string(),
                version: Some("1.0.0".to_string()),
                schema_url: None,
                attributes: vec![],
            },
            dropped_attributes_count: 0,
            dropped_events_count: 0,
            dropped_links_count: 0,
        }
    }

    fn create_test_resource() -> foundation::OtelResource {
        foundation::OtelResource {
            attributes: vec![
                foundation::Attribute {
                    key: "service.name".to_string(),
                    value: foundation::AttributeValue::String("test-service".to_string()),
                },
            ],
            schema_url: None,
        }
    }

    #[test]
    fn test_protobuf_serialization_basic() {
        let span = create_test_span();
        let resource = create_test_resource();

        let result = serialize_to_protobuf(vec![span], resource);
        assert!(result.is_ok());

        let protobuf_bytes = result.unwrap();
        // Verify we got non-empty bytes
        assert!(!protobuf_bytes.is_empty());
    }

    #[test]
    fn test_protobuf_serialization_size_reasonable() {
        let span = create_test_span();
        let resource = create_test_resource();

        let result = serialize_to_protobuf(vec![span], resource);
        assert!(result.is_ok());

        let protobuf_bytes = result.unwrap();
        // Protobuf should be compact - a single simple span should be < 500 bytes
        assert!(protobuf_bytes.len() < 500);
    }

    #[test]
    fn test_protobuf_with_multiple_spans() {
        let span1 = create_test_span();
        let mut span2 = create_test_span();
        span2.name = "span-2".to_string();

        let resource = create_test_resource();
        let result = serialize_to_protobuf(vec![span1, span2], resource);
        assert!(result.is_ok());

        let protobuf_bytes = result.unwrap();
        assert!(!protobuf_bytes.is_empty());
    }

    #[test]
    fn test_protobuf_with_events() {
        let mut span = create_test_span();
        span.events = vec![
            trace::SpanEvent {
                name: "event1".to_string(),
                timestamp: 1500000,
                attributes: vec![
                    foundation::Attribute {
                        key: "event.attr".to_string(),
                        value: foundation::AttributeValue::String("value".to_string()),
                    },
                ],
            },
        ];

        let resource = create_test_resource();
        let result = serialize_to_protobuf(vec![span], resource);
        assert!(result.is_ok());
    }

    #[test]
    fn test_protobuf_with_dropped_counts() {
        let mut span = create_test_span();
        // Encode dropped counts in trace_state
        span.context.trace_state = "dropped=5:10:2".to_string();

        let resource = create_test_resource();
        let result = serialize_to_protobuf(vec![span], resource);
        assert!(result.is_ok());

        // Verify serialization succeeds - actual dropped count validation
        // would require decoding the protobuf
        let protobuf_bytes = result.unwrap();
        assert!(!protobuf_bytes.is_empty());
    }

    #[test]
    fn test_convert_span_kind() {
        assert_eq!(convert_span_kind(&trace::SpanKind::Internal), 1);
        assert_eq!(convert_span_kind(&trace::SpanKind::Server), 2);
        assert_eq!(convert_span_kind(&trace::SpanKind::Client), 3);
        assert_eq!(convert_span_kind(&trace::SpanKind::Producer), 4);
        assert_eq!(convert_span_kind(&trace::SpanKind::Consumer), 5);
    }

    #[test]
    fn test_convert_status() {
        let status_ok = convert_status(&trace::SpanStatus::Ok);
        assert_eq!(status_ok.code, 1); // STATUS_CODE_OK

        let status_unset = convert_status(&trace::SpanStatus::Unset);
        assert_eq!(status_unset.code, 0); // STATUS_CODE_UNSET

        let status_error = convert_status(&trace::SpanStatus::Error("test error".to_string()));
        assert_eq!(status_error.code, 2); // STATUS_CODE_ERROR
        assert_eq!(status_error.message, "test error");
    }
}
