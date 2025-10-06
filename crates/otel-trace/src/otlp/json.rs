//! OTLP JSON serialization.
//!
//! Handles conversion of WIT telemetry types to OTLP JSON format using `serde_json`.
//!
//! # JSON Structure
//!
//! The output follows the OTLP/JSON specification with camelCase field names:
//! ```json
//! {
//!   "resourceSpans": [{
//!     "resource": {
//!       "attributes": [{"key": "service.name", "value": {"stringValue": "my-service"}}]
//!     },
//!     "scopeSpans": [{
//!       "scope": {"name": "my-library", "version": "1.0.0"},
//!       "spans": [{
//!         "traceId": "0af7651916cd43dd8448eb211c80319c",
//!         "spanId": "b7ad6b7169203331",
//!         "name": "my-operation",
//!         "kind": 1,
//!         "startTimeUnixNano": "1000000",
//!         "endTimeUnixNano": "2000000",
//!         "flags": 1,
//!         "attributes": [...],
//!         "events": [...],
//!         "links": [...]
//!       }]
//!     }]
//!   }]
//! }
//! ```
//!
//! # Key Features
//!
//! - **Base64 Encoding**: Trace IDs and span IDs are hex-encoded strings (OTLP/JSON spec)
//! - **Dropped Counts**: Decodes dropped counts from trace_state (Issue #4 workaround)
//! - **Flags Field**: Includes sampling flags in span output (Issue #6 fix)
//! - **Attribute Types**: Supports all OTLP attribute value types (string, int, bool, float, arrays, kvlist)
//! - **Scope Grouping**: Groups spans by instrumentation scope as required by OTLP

use crate::bindings::exports::wasi::otel_sdk::trace;
use crate::bindings::wasi::otel_sdk::foundation;
use crate::error::SerializationError;

use serde::{Serialize, Deserialize};
use base64::{Engine as _, engine::general_purpose};

/// Serializes spans to OTLP JSON format.
///
/// Converts WIT span data into OTLP-compliant JSON bytes ready for HTTP transport.
/// The output follows the OpenTelemetry Protocol JSON encoding specification.
///
/// # Arguments
///
/// * `spans` - Vector of span data to serialize
/// * `service_resource` - Service resource attributes (appears in `resource` field)
///
/// # Returns
///
/// - `Ok(Vec<u8>)` - UTF-8 encoded JSON bytes
/// - `Err(SerializationError::JsonEncoding)` - serde_json serialization failed
///
/// # OTLP Compliance
///
/// - Uses camelCase for all field names per OTLP/JSON spec
/// - Encodes trace_id and span_id as hex strings
/// - Includes all required OTLP fields
/// - Groups spans by instrumentation scope
/// - Preserves all attribute types (arrays, nested kvlists, etc.)
///
/// # Examples
///
/// ```no_run
/// # use otel_trace::otlp::json::serialize_to_json;
/// # use otel_trace::bindings::exports::wasi::otel_sdk::trace::SpanData;
/// # use otel_trace::bindings::wasi::otel_sdk::foundation::OtelResource;
/// # fn example(spans: Vec<SpanData>, resource: OtelResource) -> Result<(), Box<dyn std::error::Error>> {
/// let json_bytes = serialize_to_json(spans, resource)?;
/// let json_str = String::from_utf8(json_bytes)?;
/// println!("{}", json_str);  // Pretty-printable JSON
/// # Ok(())
/// # }
/// ```
pub fn serialize_to_json(
    spans: Vec<trace::SpanData>,
    service_resource: foundation::OtelResource,
) -> Result<Vec<u8>, SerializationError> {
    let request = build_otlp_json_request(spans, service_resource);

    serde_json::to_vec(&request)
        .map_err(SerializationError::from)
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
        trace_state: if span.context.trace_state.is_empty() {
            None
        } else {
            Some(span.context.trace_state.clone())
        },
        parent_span_id: span.parent_span_id.map(|id| hex_encode(&id)),
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
    fn test_json_serialization_basic() {
        let span = create_test_span();
        let resource = create_test_resource();

        let result = serialize_to_json(vec![span], resource);
        assert!(result.is_ok());

        let json_bytes = result.unwrap();
        let json_str = String::from_utf8(json_bytes).expect("Invalid UTF-8");
        let json_value: serde_json::Value = serde_json::from_str(&json_str)
            .expect("Invalid JSON");

        // Verify top-level structure
        assert!(json_value.get("resourceSpans").is_some());
    }

    #[test]
    fn test_json_contains_flags_field() {
        let span = create_test_span();
        let resource = create_test_resource();

        let result = serialize_to_json(vec![span], resource);
        assert!(result.is_ok());

        let json_str = String::from_utf8(result.unwrap()).unwrap();
        let json_value: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Navigate to the span
        let spans = json_value["resourceSpans"][0]["scopeSpans"][0]["spans"].as_array().unwrap();
        assert_eq!(spans.len(), 1);

        // Verify flags field is present (Issue #6 fix)
        assert!(spans[0].get("flags").is_some());
        assert_eq!(spans[0]["flags"].as_u64().unwrap(), 1); // Sampled flag
    }

    #[test]
    fn test_json_with_multiple_attribute_types() {
        let mut span = create_test_span();
        span.attributes = vec![
            foundation::Attribute {
                key: "string.attr".to_string(),
                value: foundation::AttributeValue::String("value".to_string()),
            },
            foundation::Attribute {
                key: "int.attr".to_string(),
                value: foundation::AttributeValue::Int64(42),
            },
            foundation::Attribute {
                key: "bool.attr".to_string(),
                value: foundation::AttributeValue::Bool(true),
            },
            foundation::Attribute {
                key: "float.attr".to_string(),
                value: foundation::AttributeValue::Float64(3.14),
            },
            foundation::Attribute {
                key: "array.attr".to_string(),
                value: foundation::AttributeValue::ArrayString(vec![
                    "a".to_string(),
                    "b".to_string(),
                ]),
            },
        ];

        let resource = create_test_resource();
        let result = serialize_to_json(vec![span], resource);
        assert!(result.is_ok());

        // Verify it's valid JSON
        let json_str = String::from_utf8(result.unwrap()).unwrap();
        let _: serde_json::Value = serde_json::from_str(&json_str).expect("Invalid JSON");
    }

    #[test]
    fn test_json_with_events() {
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
        let result = serialize_to_json(vec![span], resource);
        assert!(result.is_ok());

        let json_str = String::from_utf8(result.unwrap()).unwrap();
        let json_value: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let events = json_value["resourceSpans"][0]["scopeSpans"][0]["spans"][0]["events"]
            .as_array().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["name"].as_str().unwrap(), "event1");
    }

    #[test]
    fn test_json_with_dropped_counts() {
        let mut span = create_test_span();
        // Set dropped counts directly in the span data
        span.dropped_attributes_count = 2;
        span.dropped_events_count = 3;
        span.dropped_links_count = 1;

        let resource = create_test_resource();
        let result = serialize_to_json(vec![span], resource);
        assert!(result.is_ok());

        let json_str = String::from_utf8(result.unwrap()).unwrap();
        let json_value: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let span_json = &json_value["resourceSpans"][0]["scopeSpans"][0]["spans"][0];

        // Verify dropped counts are properly serialized
        assert_eq!(span_json["droppedAttributesCount"].as_u64().unwrap(), 2);
        assert_eq!(span_json["droppedEventsCount"].as_u64().unwrap(), 3);
        assert_eq!(span_json["droppedLinksCount"].as_u64().unwrap(), 1);
    }
}
