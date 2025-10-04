//! Context provider for OpenTelemetry SDK.

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        world: "context-provider",
        generate_all,
    });
}

use bindings::exports::wasi::otel_sdk::context::{
    ContextCarrier, ContextResult, Guest, SpanContext,
};

mod context;

pub struct Component;

impl Guest for Component {
    /// Set the active span context for this component
    fn set_active_context(context: SpanContext) {
        context::set_active_context(context);
    }

    /// Get the currently active span context
    fn get_active_context() -> Option<SpanContext> {
        context::get_active_context()
    }

    /// Clear the active context
    fn clear_active_context() {
        context::clear_active_context();
    }

    /// Extract span context from context carriers
    fn extract_context(carriers: Vec<ContextCarrier>) -> ContextResult {
        context::extract_context(carriers)
    }

    /// Inject span context into context carriers
    fn inject_context(context: SpanContext, _carriers: Vec<ContextCarrier>) -> Vec<ContextCarrier> {
        context::inject_context(&context)
    }

    /// Create context carriers from span context
    fn create_carriers(context: SpanContext) -> Vec<ContextCarrier> {
        context::inject_context(&context)
    }

    /// Parse W3C TraceContext traceparent header format
    fn parse_traceparent(traceparent: String) -> ContextResult {
        context::parse_traceparent(traceparent)
    }

    /// Format span context as W3C TraceContext traceparent
    fn format_traceparent(context: SpanContext) -> String {
        context::format_traceparent(&context)
    }

    /// Parse W3C TraceState header format
    fn parse_tracestate(tracestate: String) -> Result<String, String> {
        context::parse_tracestate(tracestate)
    }

    /// Format trace state as W3C TraceState header
    fn format_tracestate(tracestate: String) -> String {
        context::format_tracestate(tracestate)
    }

    /// Validate W3C TraceContext format compliance
    fn validate_traceparent(traceparent: String) -> Result<(), String> {
        if context::validate_traceparent(traceparent) {
            Ok(())
        } else {
            Err("Invalid traceparent format".to_string())
        }
    }

    /// Generate random trace ID (16 bytes)
    fn generate_trace_id() -> Vec<u8> {
        context::generate_trace_id()
    }

    /// Generate random span ID (8 bytes)
    fn generate_span_id() -> Vec<u8> {
        context::generate_span_id()
    }

    /// Create root span context (no parent)
    fn create_root_context(trace_id: Vec<u8>, span_id: Vec<u8>, trace_flags: u8) -> SpanContext {
        context::create_root_context(&trace_id, &span_id, trace_flags)
    }

    /// Create child span context from parent
    fn create_child_context(parent: SpanContext, span_id: Vec<u8>) -> SpanContext {
        context::create_child_context(&parent, &span_id)
    }

    /// Check if span context is valid
    fn is_valid_context(context: SpanContext) -> bool {
        context::is_valid_context(&context)
    }

    /// Check if context indicates sampling
    fn is_sampled(context: SpanContext) -> bool {
        context::is_sampled(&context)
    }

    /// Set sampling flag in context
    fn set_sampled(context: SpanContext, sampled: bool) -> SpanContext {
        context::set_sampled(&context, sampled)
    }
}

bindings::export!(Component with_types_in bindings);