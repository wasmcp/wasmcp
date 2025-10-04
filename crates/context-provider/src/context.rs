//! W3C TraceContext utilities and context management implementation.

use crate::bindings::exports::wasi::otel_sdk::context::{ContextCarrier, ContextResult, SpanContext};
use std::cell::RefCell;

/// Thread-local storage for the active context
thread_local! {
    static ACTIVE_CONTEXT: RefCell<Option<SpanContext>> = RefCell::new(None);
}

/// Set the active span context for this component
pub fn set_active_context(context: SpanContext) {
    ACTIVE_CONTEXT.with(|c| {
        *c.borrow_mut() = Some(context);
    });
}

/// Get the currently active span context
pub fn get_active_context() -> Option<SpanContext> {
    ACTIVE_CONTEXT.with(|c| c.borrow().clone())
}

/// Clear the active context
pub fn clear_active_context() {
    ACTIVE_CONTEXT.with(|c| {
        *c.borrow_mut() = None;
    });
}

/// Extract span context from context carriers
pub fn extract_context(carriers: Vec<ContextCarrier>) -> ContextResult {
    // Look for traceparent header
    let traceparent = carriers
        .iter()
        .find(|c| c.key.to_lowercase() == "traceparent")
        .map(|c| c.value.clone());

    // Look for tracestate header
    let tracestate = carriers
        .iter()
        .find(|c| c.key.to_lowercase() == "tracestate")
        .map(|c| c.value.clone())
        .unwrap_or_default();

    match traceparent {
        Some(tp) => match parse_traceparent_internal(&tp) {
            Ok(mut context) => {
                context.trace_state = tracestate;
                context.is_remote = true;
                ContextResult::Success(context)
            }
            Err(e) => ContextResult::Invalid(e),
        },
        None => ContextResult::NotFound,
    }
}

/// Inject span context into context carriers
pub fn inject_context(context: SpanContext, mut carriers: Vec<ContextCarrier>) -> Vec<ContextCarrier> {
    // Remove any existing traceparent/tracestate
    carriers.retain(|c| {
        let key_lower = c.key.to_lowercase();
        key_lower != "traceparent" && key_lower != "tracestate"
    });

    // Add new traceparent
    carriers.push(ContextCarrier {
        key: "traceparent".to_string(),
        value: format_traceparent(context.clone()),
    });

    // Add tracestate if not empty
    if !context.trace_state.is_empty() {
        carriers.push(ContextCarrier {
            key: "tracestate".to_string(),
            value: context.trace_state,
        });
    }

    carriers
}

/// Create context carriers from span context
pub fn create_carriers(context: SpanContext) -> Vec<ContextCarrier> {
    let mut carriers = vec![
        ContextCarrier {
            key: "traceparent".to_string(),
            value: format_traceparent(context.clone()),
        },
    ];

    if !context.trace_state.is_empty() {
        carriers.push(ContextCarrier {
            key: "tracestate".to_string(),
            value: context.trace_state,
        });
    }

    carriers
}

/// Parse W3C TraceContext traceparent header format
pub fn parse_traceparent(traceparent: String) -> ContextResult {
    match parse_traceparent_internal(&traceparent) {
        Ok(context) => ContextResult::Success(context),
        Err(e) => ContextResult::Invalid(e),
    }
}

/// Internal traceparent parsing logic
fn parse_traceparent_internal(traceparent: &str) -> Result<SpanContext, String> {
    // W3C format: version-trace_id-span_id-trace_flags
    // Example: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01

    let parts: Vec<&str> = traceparent.split('-').collect();
    if parts.len() != 4 {
        return Err("Invalid traceparent format: expected 4 parts separated by '-'".to_string());
    }

    // Check version
    if parts[0] != "00" {
        return Err(format!("Unsupported traceparent version: {}", parts[0]));
    }

    // Parse trace ID (32 hex chars = 16 bytes)
    if parts[1].len() != 32 {
        return Err("Invalid trace ID: must be 32 hex characters".to_string());
    }
    let trace_id = hex::decode(parts[1])
        .map_err(|_| "Invalid trace ID: not valid hex".to_string())?;

    // Parse span ID (16 hex chars = 8 bytes)
    if parts[2].len() != 16 {
        return Err("Invalid span ID: must be 16 hex characters".to_string());
    }
    let span_id = hex::decode(parts[2])
        .map_err(|_| "Invalid span ID: not valid hex".to_string())?;

    // Parse trace flags (2 hex chars = 1 byte)
    if parts[3].len() != 2 {
        return Err("Invalid trace flags: must be 2 hex characters".to_string());
    }
    let trace_flags = u8::from_str_radix(parts[3], 16)
        .map_err(|_| "Invalid trace flags: not valid hex".to_string())?;

    Ok(SpanContext {
        trace_id,
        span_id,
        trace_flags,
        trace_state: String::new(),
        is_remote: false,
    })
}

/// Format span context as W3C TraceContext traceparent
pub fn format_traceparent(context: SpanContext) -> String {
    format!(
        "00-{}-{}-{:02x}",
        hex::encode(&context.trace_id),
        hex::encode(&context.span_id),
        context.trace_flags
    )
}

/// Parse W3C TraceState header format
pub fn parse_tracestate(tracestate: String) -> Result<String, String> {
    // TraceState format: key1=value1,key2=value2
    // Basic validation - ensure it's properly formatted

    if tracestate.is_empty() {
        return Ok(String::new());
    }

    // Validate each key-value pair
    for pair in tracestate.split(',') {
        let parts: Vec<&str> = pair.split('=').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid tracestate pair: '{}'", pair));
        }

        let key = parts[0].trim();
        let value = parts[1].trim();

        // Basic key validation (simplified W3C rules)
        if key.is_empty() || key.len() > 256 {
            return Err(format!("Invalid tracestate key: '{}'", key));
        }

        // Basic value validation
        if value.is_empty() || value.len() > 256 {
            return Err(format!("Invalid tracestate value for key '{}'", key));
        }
    }

    Ok(tracestate)
}

/// Format trace state as W3C TraceState header
pub fn format_tracestate(tracestate: String) -> String {
    // Already in the correct format, just return it
    tracestate
}

/// Validate W3C TraceContext format compliance
pub fn validate_traceparent(traceparent: String) -> Result<(), String> {
    parse_traceparent_internal(&traceparent)?;
    Ok(())
}

/// Generate random trace ID (16 bytes)
pub fn generate_trace_id() -> Vec<u8> {
    // Generate 16 random bytes for trace ID
    let mut trace_id = vec![0u8; 16];

    // Use rand crate for random generation
    use rand::Rng;
    let mut rng = rand::thread_rng();
    rng.fill(&mut trace_id[..]);

    // Ensure trace ID is not all zeros (invalid per spec)
    if trace_id.iter().all(|&b| b == 0) {
        trace_id[0] = 1;
    }

    trace_id
}

/// Generate random span ID (8 bytes)
pub fn generate_span_id() -> Vec<u8> {
    // Generate 8 random bytes for span ID
    let mut span_id = vec![0u8; 8];

    // Use rand crate for random generation
    use rand::Rng;
    let mut rng = rand::thread_rng();
    rng.fill(&mut span_id[..]);

    // Ensure span ID is not all zeros (invalid per spec)
    if span_id.iter().all(|&b| b == 0) {
        span_id[0] = 1;
    }

    span_id
}

/// Create root span context (no parent)
pub fn create_root_context(trace_id: Vec<u8>, span_id: Vec<u8>, trace_flags: u8) -> SpanContext {
    SpanContext {
        trace_id,
        span_id,
        trace_flags,
        trace_state: String::new(),
        is_remote: false,
    }
}

/// Create child span context from parent
pub fn create_child_context(parent: SpanContext, span_id: Vec<u8>) -> SpanContext {
    SpanContext {
        trace_id: parent.trace_id,
        span_id,
        trace_flags: parent.trace_flags,
        trace_state: parent.trace_state,
        is_remote: false,
    }
}

/// Check if span context is valid
pub fn is_valid_context(context: SpanContext) -> bool {
    // Trace ID must be exactly 16 bytes and not all zeros
    if context.trace_id.len() != 16 || context.trace_id.iter().all(|&b| b == 0) {
        return false;
    }

    // Span ID must be exactly 8 bytes and not all zeros
    if context.span_id.len() != 8 || context.span_id.iter().all(|&b| b == 0) {
        return false;
    }

    true
}

/// Check if context indicates sampling
pub fn is_sampled(context: SpanContext) -> bool {
    // Check the sampled bit (bit 0) in trace flags
    (context.trace_flags & 0x01) != 0
}

/// Set sampling flag in context
pub fn set_sampled(mut context: SpanContext, sampled: bool) -> SpanContext {
    if sampled {
        // Set the sampled bit
        context.trace_flags |= 0x01;
    } else {
        // Clear the sampled bit
        context.trace_flags &= !0x01;
    }
    context
}