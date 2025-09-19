// Minimal OTEL implementation that just satisfies the WASI OTEL interface
// without complex external dependencies that cause validation errors

use std::cell::RefCell;
use std::collections::HashMap;

mod bindings;
use bindings::exports::wasmcp::otel_exporter::api::{Guest, GuestSpan, SpanBorrow};
use bindings::exports::wasi::otel::tracing::{Guest as WasiOtelGuest, SpanData, SpanContext as WasiSpanContext};
use bindings::exports::wasi::otel::tracing::TraceFlags;

// Simple span implementation
#[derive(Debug)]
pub struct SimpleSpan {
    name: String,
    context: WasiSpanContext,
    events: RefCell<Vec<String>>,
}

impl SimpleSpan {
    fn new(name: String, parent_context: Option<&WasiSpanContext>) -> Self {
        // Generate a simple trace/span ID
        let trace_id = parent_context
            .map(|p| p.trace_id.clone())
            .unwrap_or_else(|| "12345678901234567890123456789012".to_string());
        let span_id = format!("{:016x}", fastrand::u64(..));

        SimpleSpan {
            name,
            context: WasiSpanContext {
                trace_id,
                span_id,
                trace_flags: TraceFlags::empty(),
                is_remote: false,
                trace_state: vec![],
            },
            events: RefCell::new(vec![]),
        }
    }
}

impl GuestSpan for SimpleSpan {
    fn new(name: String, parent_context: Option<WasiSpanContext>) -> Self {
        SimpleSpan::new(name, parent_context.as_ref())
    }

    fn add_event(&self, name: String) {
        self.events.borrow_mut().push(name);
        println!("[SIMPLE-OTEL] Event: {} on span: {}", name, self.name);
    }

    fn finish(&self) {
        println!("[SIMPLE-OTEL] Finished span: {} with {} events",
                self.name, self.events.borrow().len());
    }
}

// Component implementation
struct Component;

impl Guest for Component {
    type Span = SimpleSpan;

    fn get_context(s: SpanBorrow<'_>) -> WasiSpanContext {
        let span_impl = s.get::<SimpleSpan>();
        span_impl.context.clone()
    }
}

impl WasiOtelGuest for Component {
    fn on_start(span: SpanData, parent: WasiSpanContext) {
        println!("[WASI-OTEL] Span started: {} (parent: {})", span.name, parent.span_id);
    }

    fn on_end(span: SpanData) {
        println!("[WASI-OTEL] Span ended: {}", span.name);
    }

    fn current_span_context() -> WasiSpanContext {
        WasiSpanContext {
            trace_id: "00000000000000000000000000000000".to_string(),
            span_id: "0000000000000000".to_string(),
            trace_flags: TraceFlags::empty(),
            is_remote: false,
            trace_state: vec![],
        }
    }
}

bindings::export!(Component with_types_in bindings);