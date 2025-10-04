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
        }));

        Self {
            inner,
        }
    }

    /// Get a clone of the inner Arc for registry storage
    pub fn inner_arc(&self) -> Arc<Mutex<SpanInner>> {
        self.inner.clone()
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

        // Add new attribute
        inner.attributes.push(foundation::Attribute { key, value });
    }

    fn add_event(&self, name: String, attributes: Vec<foundation::Attribute>, timestamp: Option<u64>) {
        let mut inner = self.inner.lock().unwrap();
        if !inner.is_recording {
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

            // Build and return span data
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
            }
        } else {
            // Fallback if span not found in registry
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