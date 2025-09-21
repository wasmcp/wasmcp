use super::{Protocol, ProtocolError};
use crate::SpanImpl;

// Use official opentelemetry-proto generated types
use opentelemetry_proto::tonic::trace::v1 as otlp_trace;
use opentelemetry_proto::tonic::common::v1 as otlp_common;
use opentelemetry_proto::tonic::resource::v1 as otlp_resource;
use opentelemetry::trace::{SpanKind, Status};

#[derive(Debug, Clone)]
pub struct OtlpHttpConfig {
    pub content_type: String,
    pub compression: Option<CompressionType>,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone)]
pub enum CompressionType {
    Gzip,
    Deflate,
    None,
}

pub struct OtlpHttpProtocol;

impl Protocol for OtlpHttpProtocol {
    type Config = OtlpHttpConfig;

    fn serialize_span(&self, span: &SpanImpl, config: &Self::Config) -> Result<Vec<u8>, ProtocolError> {
        println!("[OTLP-HTTP] Serializing span '{}' to protobuf format", span.name());

        let proto_data = self.create_otlp_proto(span, config)?;
        println!("[OTLP-HTTP] Created protobuf data with {} resource spans", proto_data.resource_spans.len());

        // Serialize to protobuf binary format
        use prost::Message;
        let proto_bytes = proto_data.encode_to_vec();
        println!("[OTLP-HTTP] Serialized to {} bytes", proto_bytes.len());

        Ok(proto_bytes)
    }

    fn content_type(&self) -> &'static str {
        "application/x-protobuf"
    }

    fn supports_compression(&self) -> bool {
        true
    }
}

impl OtlpHttpProtocol {
    pub fn new() -> Self {
        Self
    }

    fn create_otlp_proto(&self, span: &SpanImpl, _config: &OtlpHttpConfig) -> Result<otlp_trace::TracesData, ProtocolError> {
        // Convert span context for protobuf
        let trace_id = hex::decode(span.context().trace_id.as_str())
            .map_err(|e| ProtocolError::InvalidData(format!("Invalid trace_id: {}", e)))?;
        let span_id = hex::decode(span.context().span_id.as_str())
            .map_err(|e| ProtocolError::InvalidData(format!("Invalid span_id: {}", e)))?;

        let parent_span_id = span.parent_context()
            .as_ref()
            .and_then(|ctx| hex::decode(&ctx.span_id).ok())
            .unwrap_or_default();

        // Access RefCell fields safely
        let events = span.events();
        let end_time = span.end_time_nanos();
        let status = span.status();

        println!("[OTLP-HTTP] Span has {} events, end_time: {:?}", events.len(), end_time);

        // Create protobuf span
        let proto_span = otlp_trace::Span {
            trace_id,
            span_id,
            parent_span_id,
            name: span.name().to_string(),
            kind: self.span_kind_to_proto(span.kind()),
            start_time_unix_nano: span.start_time_nanos(),
            end_time_unix_nano: end_time.unwrap_or(span.start_time_nanos() + 1_000_000),
            flags: span.context().trace_flags.bits() as u32,
            attributes: span.attributes().iter().map(|(k, v)| {
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
            status: Some(self.status_to_proto(&status)),
            links: vec![],
            dropped_attributes_count: 0,
            dropped_events_count: 0,
            dropped_links_count: 0,
            trace_state: "".to_string(),
        };

        // Create protobuf traces data with resource attributes
        Ok(otlp_trace::TracesData {
            resource_spans: vec![otlp_trace::ResourceSpans {
                resource: Some(otlp_resource::Resource {
                    attributes: vec![
                        otlp_common::KeyValue {
                            key: "service.name".to_string(),
                            value: Some(otlp_common::AnyValue {
                                value: Some(otlp_common::any_value::Value::StringValue("wasmcp-otel-exporter".to_string())),
                            }),
                        }
                    ],
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
        })
    }

    // Helper function to convert SpanKind to OTLP protobuf enum
    fn span_kind_to_proto(&self, kind: &SpanKind) -> i32 {
        match kind {
            SpanKind::Internal => 1,
            SpanKind::Server => 2,
            SpanKind::Client => 3,
            SpanKind::Producer => 4,
            SpanKind::Consumer => 5,
        }
    }

    // Helper function to convert Status to OTLP protobuf format
    fn status_to_proto(&self, status: &Status) -> otlp_trace::Status {
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
}

impl Default for OtlpHttpConfig {
    fn default() -> Self {
        Self {
            content_type: "application/x-protobuf".to_string(),
            compression: Some(CompressionType::None),
            timeout_ms: Some(5000),
        }
    }
}