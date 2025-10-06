use crate::bindings::exports::wasi::otel_sdk::trace;
use crate::bindings::wasi::otel_sdk::foundation;
use crate::bindings::wasi::otel_sdk::context;

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Implementation of the span resource
pub struct SpanImpl {
    inner: Arc<Mutex<SpanInner>>,
}

pub struct SpanInner {
    name: String,
    context: context::SpanContext,
    parent_span_id: Option<Vec<u8>>,
    kind: trace::SpanKind,
    start_time: u64,
    end_time: Option<u64>,
    attributes: Vec<foundation::Attribute>,
    events: Vec<trace::SpanEvent>,
    links: Vec<trace::SpanLink>,
    status: trace::SpanStatus,
    instrumentation_scope: foundation::InstrumentationScope,
    is_recording: bool,
    // Limits configuration
    max_attributes: u32,
    max_events: u32,
    max_links: u32,
    // Dropped counts for OTLP export
    dropped_attributes_count: u32,
    dropped_events_count: u32,
    dropped_links_count: u32,
}

impl SpanImpl {
    /// Create a new span with the given parameters
    pub fn new(
        name: String,
        context: context::SpanContext,
        parent_span_id: Option<Vec<u8>>,
        kind: trace::SpanKind,
        start_time: Option<u64>,
        attributes: Vec<foundation::Attribute>,
        links: Vec<trace::SpanLink>,
        instrumentation_scope: foundation::InstrumentationScope,
        max_attributes: u32,
        max_events: u32,
        max_links: u32,
    ) -> Self {
        let start = start_time.unwrap_or_else(|| current_timestamp_nanos());

        // Create the inner data
        let inner = Arc::new(Mutex::new(SpanInner {
            name,
            context,
            parent_span_id,
            kind,
            start_time: start,
            end_time: None,
            attributes,
            events: Vec::new(),
            links,
            status: trace::SpanStatus::Unset,
            instrumentation_scope,
            is_recording: true,
            max_attributes,
            max_events,
            max_links,
            dropped_attributes_count: 0,
            dropped_events_count: 0,
            dropped_links_count: 0,
        }));

        Self {
            inner,
        }
    }

    /// Get a clone of the inner Arc for registry storage
    pub fn inner_arc(&self) -> Arc<Mutex<SpanInner>> {
        self.inner.clone()
    }

    /// Get dropped counts for OTLP export
    pub fn get_dropped_counts(&self) -> (u32, u32, u32) {
        let inner = self.inner.lock().unwrap();
        (
            inner.dropped_attributes_count,
            inner.dropped_events_count,
            inner.dropped_links_count,
        )
    }
}

impl trace::GuestSpan for SpanImpl {
    fn get_context(&self) -> context::SpanContext {
        let inner = self.inner.lock().unwrap();
        inner.context.clone()
    }

    fn is_recording(&self) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.is_recording
    }

    fn set_attribute(&self, key: String, value: foundation::AttributeValue) {
        let mut inner = self.inner.lock().unwrap();
        if !inner.is_recording {
            return;
        }

        // Check if attribute already exists and update it
        for attr in &mut inner.attributes {
            if attr.key == key {
                attr.value = value;
                return;
            }
        }

        // Check if we've reached the limit
        if inner.attributes.len() >= inner.max_attributes as usize {
            inner.dropped_attributes_count += 1;
            return;
        }

        // Add new attribute
        inner.attributes.push(foundation::Attribute { key, value });
    }

    fn add_event(&self, name: String, attributes: Vec<foundation::Attribute>, timestamp: Option<u64>) {
        let mut inner = self.inner.lock().unwrap();
        if !inner.is_recording {
            return;
        }

        // Check if we've reached the limit
        if inner.events.len() >= inner.max_events as usize {
            inner.dropped_events_count += 1;
            return;
        }

        let event_timestamp = timestamp.unwrap_or_else(|| current_timestamp_nanos());

        inner.events.push(trace::SpanEvent {
            name,
            timestamp: event_timestamp,
            attributes,
        });
    }

    fn add_link(&self, link: trace::SpanLink) {
        let mut inner = self.inner.lock().unwrap();
        if !inner.is_recording {
            return;
        }

        // Check if we've reached the limit
        if inner.links.len() >= inner.max_links as usize {
            inner.dropped_links_count += 1;
            return;
        }

        inner.links.push(link);
    }

    fn set_status(&self, status: trace::SpanStatus) {
        let mut inner = self.inner.lock().unwrap();
        if !inner.is_recording {
            return;
        }

        // Only allow setting error status or initial ok status
        match (&inner.status, &status) {
            (trace::SpanStatus::Unset, _) => inner.status = status,
            (trace::SpanStatus::Ok, trace::SpanStatus::Error(_)) => inner.status = status,
            _ => {} // Ignore other status changes
        }
    }

    fn update_name(&self, name: String) {
        let mut inner = self.inner.lock().unwrap();
        if !inner.is_recording {
            return;
        }

        inner.name = name;
    }

    fn record_exception(&self, exception_type: String, message: String, stacktrace: Option<String>) {
        let mut attributes = vec![
            foundation::Attribute {
                key: "exception.type".to_string(),
                value: foundation::AttributeValue::String(exception_type),
            },
            foundation::Attribute {
                key: "exception.message".to_string(),
                value: foundation::AttributeValue::String(message),
            },
        ];

        if let Some(stack) = stacktrace {
            attributes.push(foundation::Attribute {
                key: "exception.stacktrace".to_string(),
                value: foundation::AttributeValue::String(stack),
            });
        }

        self.add_event("exception".to_string(), attributes, None);

        // Set status to error if not already set
        let inner = self.inner.lock().unwrap();
        if matches!(inner.status, trace::SpanStatus::Unset) {
            drop(inner); // Release lock before calling set_status
            self.set_status(trace::SpanStatus::Error("Exception recorded".to_string()));
        }
    }

    fn finish(span: trace::Span, end_time: Option<u64>) -> trace::SpanData {
        // Get the handle from the span wrapper
        let handle = span.handle();

        // Retrieve the span data from the registry and remove it
        if let Some(span_data) = crate::remove_span(handle) {
            let mut inner = span_data.lock().unwrap();

            // Mark span as no longer recording
            inner.is_recording = false;

            // Set end time
            inner.end_time = Some(end_time.unwrap_or_else(|| current_timestamp_nanos()));

            // Build and return span data with dropped counts
            trace::SpanData {
                name: inner.name.clone(),
                context: inner.context.clone(),
                parent_span_id: inner.parent_span_id.clone(),
                kind: inner.kind.clone(),
                start_time: inner.start_time,
                end_time: inner.end_time,
                attributes: inner.attributes.clone(),
                events: inner.events.clone(),
                links: inner.links.clone(),
                status: inner.status.clone(),
                instrumentation_scope: inner.instrumentation_scope.clone(),
                dropped_attributes_count: inner.dropped_attributes_count,
                dropped_events_count: inner.dropped_events_count,
                dropped_links_count: inner.dropped_links_count,
            }
        } else {
            // Fallback if span not found in registry
            // This should not happen in normal operation - indicates a bug
            eprintln!(
                "[otel-trace] ERROR: Span handle {} not found in registry. \
                 This indicates a bug in span lifecycle management.",
                handle
            );

            trace::SpanData {
                name: String::from("unknown"),
                context: context::SpanContext {
                    trace_id: vec![0; 16],
                    span_id: vec![0; 8],
                    trace_flags: 0,
                    trace_state: String::new(),
                    is_remote: false,
                },
                parent_span_id: None,
                kind: trace::SpanKind::Internal,
                start_time: 0,
                end_time: Some(end_time.unwrap_or_else(|| current_timestamp_nanos())),
                attributes: Vec::new(),
                events: Vec::new(),
                links: Vec::new(),
                status: trace::SpanStatus::Unset,
                instrumentation_scope: foundation::InstrumentationScope {
                    name: String::from("unknown"),
                    version: None,
                    schema_url: None,
                    attributes: Vec::new(),
                },
                dropped_attributes_count: 0,
                dropped_events_count: 0,
                dropped_links_count: 0,
            }
        }
    }
}

/// Get the current timestamp in nanoseconds since Unix epoch
fn current_timestamp_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::exports::wasi::otel_sdk::trace::GuestSpan;

    #[test]
    fn test_span_creation() {
        let context = context::SpanContext {
            trace_id: vec![1u8; 16],
            span_id: vec![2u8; 8],
            trace_flags: 0x01,
            trace_state: String::new(),
            is_remote: false,
        };

        let scope = foundation::InstrumentationScope {
            name: "test-library".to_string(),
            version: Some("1.0.0".to_string()),
            schema_url: None,
            attributes: vec![],
        };

        let span = SpanImpl::new(
            "test-span".to_string(),
            context.clone(),
            None,
            trace::SpanKind::Internal,
            Some(123456789),
            vec![],
            vec![],
            scope.clone(),
            128, // max_attributes
            128, // max_events
            128, // max_links
        );

        let inner = span.inner.lock().unwrap();
        assert_eq!(inner.name, "test-span");
        assert_eq!(inner.context.trace_id, context.trace_id);
        assert_eq!(inner.start_time, 123456789);
        assert!(inner.is_recording);
        assert!(matches!(inner.status, trace::SpanStatus::Unset));
    }

    #[test]
    fn test_span_get_context() {
        let context = context::SpanContext {
            trace_id: vec![1u8; 16],
            span_id: vec![2u8; 8],
            trace_flags: 0x01,
            trace_state: String::new(),
            is_remote: false,
        };

        let scope = foundation::InstrumentationScope {
            name: "test-library".to_string(),
            version: None,
            schema_url: None,
            attributes: vec![],
        };

        let span = SpanImpl::new(
            "test-span".to_string(),
            context.clone(),
            None,
            trace::SpanKind::Internal,
            None,
            vec![],
            vec![],
            scope,
            128, // max_attributes
            128, // max_events
            128, // max_links
        );

        let retrieved_context = span.get_context();
        assert_eq!(retrieved_context.trace_id, context.trace_id);
        assert_eq!(retrieved_context.span_id, context.span_id);
    }

    #[test]
    fn test_span_is_recording() {
        let context = context::SpanContext {
            trace_id: vec![1u8; 16],
            span_id: vec![2u8; 8],
            trace_flags: 0x01,
            trace_state: String::new(),
            is_remote: false,
        };

        let scope = foundation::InstrumentationScope {
            name: "test-library".to_string(),
            version: None,
            schema_url: None,
            attributes: vec![],
        };

        let span = SpanImpl::new(
            "test-span".to_string(),
            context,
            None,
            trace::SpanKind::Internal,
            None,
            vec![],
            vec![],
            scope,
            128, // max_attributes
            128, // max_events
            128, // max_links
        );

        assert!(span.is_recording());
    }

    #[test]
    fn test_span_set_attribute() {
        let context = context::SpanContext {
            trace_id: vec![1u8; 16],
            span_id: vec![2u8; 8],
            trace_flags: 0x01,
            trace_state: String::new(),
            is_remote: false,
        };

        let scope = foundation::InstrumentationScope {
            name: "test-library".to_string(),
            version: None,
            schema_url: None,
            attributes: vec![],
        };

        let span = SpanImpl::new(
            "test-span".to_string(),
            context,
            None,
            trace::SpanKind::Internal,
            None,
            vec![],
            vec![],
            scope,
            128, // max_attributes
            128, // max_events
            128, // max_links
        );

        span.set_attribute(
            "key1".to_string(),
            foundation::AttributeValue::String("value1".to_string()),
        );

        let inner = span.inner.lock().unwrap();
        assert_eq!(inner.attributes.len(), 1);
        assert_eq!(inner.attributes[0].key, "key1");
    }

    #[test]
    fn test_span_set_attribute_update_existing() {
        let context = context::SpanContext {
            trace_id: vec![1u8; 16],
            span_id: vec![2u8; 8],
            trace_flags: 0x01,
            trace_state: String::new(),
            is_remote: false,
        };

        let scope = foundation::InstrumentationScope {
            name: "test-library".to_string(),
            version: None,
            schema_url: None,
            attributes: vec![],
        };

        let span = SpanImpl::new(
            "test-span".to_string(),
            context,
            None,
            trace::SpanKind::Internal,
            None,
            vec![],
            vec![],
            scope,
            128, // max_attributes
            128, // max_events
            128, // max_links
        );

        span.set_attribute(
            "key1".to_string(),
            foundation::AttributeValue::String("value1".to_string()),
        );
        span.set_attribute(
            "key1".to_string(),
            foundation::AttributeValue::String("value2".to_string()),
        );

        let inner = span.inner.lock().unwrap();
        assert_eq!(inner.attributes.len(), 1);
        assert_eq!(inner.attributes[0].key, "key1");
        match &inner.attributes[0].value {
            foundation::AttributeValue::String(s) => assert_eq!(s, "value2"),
            _ => panic!("Expected String value"),
        }
    }

    #[test]
    fn test_span_add_event() {
        let context = context::SpanContext {
            trace_id: vec![1u8; 16],
            span_id: vec![2u8; 8],
            trace_flags: 0x01,
            trace_state: String::new(),
            is_remote: false,
        };

        let scope = foundation::InstrumentationScope {
            name: "test-library".to_string(),
            version: None,
            schema_url: None,
            attributes: vec![],
        };

        let span = SpanImpl::new(
            "test-span".to_string(),
            context,
            None,
            trace::SpanKind::Internal,
            None,
            vec![],
            vec![],
            scope,
            128, // max_attributes
            128, // max_events
            128, // max_links
        );

        span.add_event(
            "test-event".to_string(),
            vec![foundation::Attribute {
                key: "event-key".to_string(),
                value: foundation::AttributeValue::String("event-value".to_string()),
            }],
            Some(987654321),
        );

        let inner = span.inner.lock().unwrap();
        assert_eq!(inner.events.len(), 1);
        assert_eq!(inner.events[0].name, "test-event");
        assert_eq!(inner.events[0].timestamp, 987654321);
        assert_eq!(inner.events[0].attributes.len(), 1);
    }

    #[test]
    fn test_span_set_status() {
        let context = context::SpanContext {
            trace_id: vec![1u8; 16],
            span_id: vec![2u8; 8],
            trace_flags: 0x01,
            trace_state: String::new(),
            is_remote: false,
        };

        let scope = foundation::InstrumentationScope {
            name: "test-library".to_string(),
            version: None,
            schema_url: None,
            attributes: vec![],
        };

        let span = SpanImpl::new(
            "test-span".to_string(),
            context,
            None,
            trace::SpanKind::Internal,
            None,
            vec![],
            vec![],
            scope,
            128, // max_attributes
            128, // max_events
            128, // max_links
        );

        span.set_status(trace::SpanStatus::Ok);
        {
            let inner = span.inner.lock().unwrap();
            assert!(matches!(inner.status, trace::SpanStatus::Ok));
        }

        // Should allow error status to override ok status
        span.set_status(trace::SpanStatus::Error("test error".to_string()));
        {
            let inner = span.inner.lock().unwrap();
            match &inner.status {
                trace::SpanStatus::Error(msg) => assert_eq!(msg, "test error"),
                _ => panic!("Expected Error status"),
            }
        }
    }

    #[test]
    fn test_span_update_name() {
        let context = context::SpanContext {
            trace_id: vec![1u8; 16],
            span_id: vec![2u8; 8],
            trace_flags: 0x01,
            trace_state: String::new(),
            is_remote: false,
        };

        let scope = foundation::InstrumentationScope {
            name: "test-library".to_string(),
            version: None,
            schema_url: None,
            attributes: vec![],
        };

        let span = SpanImpl::new(
            "original-name".to_string(),
            context,
            None,
            trace::SpanKind::Internal,
            None,
            vec![],
            vec![],
            scope,
            128, // max_attributes
            128, // max_events
            128, // max_links
        );

        span.update_name("updated-name".to_string());

        let inner = span.inner.lock().unwrap();
        assert_eq!(inner.name, "updated-name");
    }

    #[test]
    fn test_span_record_exception() {
        let context = context::SpanContext {
            trace_id: vec![1u8; 16],
            span_id: vec![2u8; 8],
            trace_flags: 0x01,
            trace_state: String::new(),
            is_remote: false,
        };

        let scope = foundation::InstrumentationScope {
            name: "test-library".to_string(),
            version: None,
            schema_url: None,
            attributes: vec![],
        };

        let span = SpanImpl::new(
            "test-span".to_string(),
            context,
            None,
            trace::SpanKind::Internal,
            None,
            vec![],
            vec![],
            scope,
            128, // max_attributes
            128, // max_events
            128, // max_links
        );

        span.record_exception(
            "TestException".to_string(),
            "Something went wrong".to_string(),
            Some("line 1\nline 2".to_string()),
        );

        let inner = span.inner.lock().unwrap();
        assert_eq!(inner.events.len(), 1);
        assert_eq!(inner.events[0].name, "exception");
        assert_eq!(inner.events[0].attributes.len(), 3);

        // Should set status to error
        match &inner.status {
            trace::SpanStatus::Error(msg) => assert_eq!(msg, "Exception recorded"),
            _ => panic!("Expected Error status"),
        }
    }

    #[test]
    fn test_current_timestamp_nanos() {
        let timestamp = current_timestamp_nanos();
        assert!(timestamp > 0, "Timestamp should be positive");

        // Should be a reasonable timestamp (after 2020)
        let min_timestamp = 1577836800_000_000_000u64; // 2020-01-01
        assert!(timestamp > min_timestamp, "Timestamp should be after 2020");
    }
}