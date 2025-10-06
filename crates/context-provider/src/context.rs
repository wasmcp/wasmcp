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
pub fn inject_context(context: &SpanContext) -> Vec<ContextCarrier> {
    let mut carriers = Vec::new();

    // Add traceparent header
    let traceparent_value = format_traceparent_internal(context);
    carriers.push(ContextCarrier {
        key: "traceparent".to_string(),
        value: traceparent_value,
    });

    // Add tracestate header if present
    if !context.trace_state.is_empty() {
        carriers.push(ContextCarrier {
            key: "tracestate".to_string(),
            value: context.trace_state.clone(),
        });
    }

    carriers
}

/// Create context carriers from W3C TraceContext headers
pub fn create_carriers(traceparent: String, tracestate: Option<String>) -> Vec<ContextCarrier> {
    let mut carriers = vec![ContextCarrier {
        key: "traceparent".to_string(),
        value: traceparent,
    }];

    if let Some(ts) = tracestate {
        if !ts.is_empty() {
            carriers.push(ContextCarrier {
                key: "tracestate".to_string(),
                value: ts,
            });
        }
    }

    carriers
}

/// Parse traceparent header (internal implementation)
fn parse_traceparent_internal(traceparent: &str) -> Result<SpanContext, String> {
    // Format: version-trace_id-span_id-flags
    let parts: Vec<&str> = traceparent.split('-').collect();

    if parts.len() != 4 {
        return Err(format!(
            "Invalid traceparent format: expected 4 parts, got {}",
            parts.len()
        ));
    }

    // Parse version
    let version = parts[0];
    if version != "00" {
        return Err(format!("Unsupported traceparent version: {}", version));
    }

    // Parse trace_id (32 hex chars = 16 bytes)
    let trace_id = hex::decode(parts[1]).map_err(|e| format!("Invalid trace_id hex: {}", e))?;
    if trace_id.len() != 16 {
        return Err(format!(
            "Invalid trace_id length: expected 16 bytes, got {}",
            trace_id.len()
        ));
    }

    // Check for all-zeros trace_id
    if trace_id.iter().all(|&b| b == 0) {
        return Err("trace_id cannot be all zeros".to_string());
    }

    // Parse span_id (16 hex chars = 8 bytes)
    let span_id = hex::decode(parts[2]).map_err(|e| format!("Invalid span_id hex: {}", e))?;
    if span_id.len() != 8 {
        return Err(format!(
            "Invalid span_id length: expected 8 bytes, got {}",
            span_id.len()
        ));
    }

    // Check for all-zeros span_id
    if span_id.iter().all(|&b| b == 0) {
        return Err("span_id cannot be all zeros".to_string());
    }

    // Parse flags (2 hex chars = 1 byte)
    let flags_str = parts[3];
    if flags_str.len() != 2 {
        return Err(format!("Invalid flags length: expected 2 chars, got {}", flags_str.len()));
    }
    let flags = u8::from_str_radix(flags_str, 16)
        .map_err(|e| format!("Invalid flags hex: {}", e))?;

    Ok(SpanContext {
        trace_id,
        span_id,
        trace_flags: flags,
        trace_state: String::new(), // Will be filled in by caller
        is_remote: false,           // Will be set by caller
    })
}

/// Format traceparent header (internal implementation)
fn format_traceparent_internal(context: &SpanContext) -> String {
    format!(
        "00-{}-{}-{:02x}",
        hex::encode(&context.trace_id),
        hex::encode(&context.span_id),
        context.trace_flags
    )
}

/// Parse traceparent header string into span context
pub fn parse_traceparent(traceparent: String) -> ContextResult {
    match parse_traceparent_internal(&traceparent) {
        Ok(context) => ContextResult::Success(context),
        Err(e) => ContextResult::Invalid(e),
    }
}

/// Format span context as traceparent header string
pub fn format_traceparent(context: &SpanContext) -> String {
    format_traceparent_internal(context)
}

/// Parse tracestate header (pass-through validation)
pub fn parse_tracestate(tracestate: String) -> Result<String, String> {
    // Basic validation: tracestate should be a list of key=value pairs separated by commas
    // We don't need to parse it, just validate format
    if tracestate.is_empty() {
        return Ok(tracestate);
    }

    // Basic check: should contain key=value pairs
    if !tracestate.contains('=') {
        return Err("Invalid tracestate format: must contain key=value pairs".to_string());
    }

    Ok(tracestate)
}

/// Format tracestate (pass-through)
pub fn format_tracestate(tracestate: String) -> String {
    tracestate
}

/// Validate traceparent format
pub fn validate_traceparent(traceparent: String) -> bool {
    parse_traceparent_internal(&traceparent).is_ok()
}

/// Generate a random trace ID (16 bytes)
pub fn generate_trace_id() -> Vec<u8> {
    crate::bindings::wasi::random::random::get_random_bytes(16)
}

/// Generate a random span ID (8 bytes)
pub fn generate_span_id() -> Vec<u8> {
    crate::bindings::wasi::random::random::get_random_bytes(8)
}

/// Create a root span context
pub fn create_root_context(trace_id: &[u8], span_id: &[u8], trace_flags: u8) -> SpanContext {
    SpanContext {
        trace_id: trace_id.to_vec(),
        span_id: span_id.to_vec(),
        trace_flags,
        trace_state: String::new(),
        is_remote: false,
    }
}

/// Create a child span context (inherits trace_id from parent)
pub fn create_child_context(parent: &SpanContext, span_id: &[u8]) -> SpanContext {
    SpanContext {
        trace_id: parent.trace_id.clone(),
        span_id: span_id.to_vec(),
        trace_flags: parent.trace_flags,
        trace_state: parent.trace_state.clone(),
        is_remote: false,
    }
}

/// Check if context is valid
pub fn is_valid_context(context: &SpanContext) -> bool {
    // Check trace_id length and non-zero
    if context.trace_id.len() != 16 || context.trace_id.iter().all(|&b| b == 0) {
        return false;
    }

    // Check span_id length and non-zero
    if context.span_id.len() != 8 || context.span_id.iter().all(|&b| b == 0) {
        return false;
    }

    true
}

/// Check if context is sampled (bit 0 of trace_flags)
pub fn is_sampled(context: &SpanContext) -> bool {
    (context.trace_flags & 0x01) != 0
}

/// Set sampled flag in context
pub fn set_sampled(context: &SpanContext, sampled: bool) -> SpanContext {
    let mut new_context = context.clone();
    if sampled {
        new_context.trace_flags |= 0x01;
    } else {
        new_context.trace_flags &= !0x01;
    }
    new_context
}

#[cfg(test)]
mod tests {
    use super::*;

    // Random generation tests only work in WASM environment where wasi:random is available
    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_generate_trace_id() {
        let trace_id = generate_trace_id();
        assert_eq!(trace_id.len(), 16, "trace_id should be 16 bytes");
        assert!(!trace_id.iter().all(|&b| b == 0), "trace_id should not be all zeros");
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_generate_span_id() {
        let span_id = generate_span_id();
        assert_eq!(span_id.len(), 8, "span_id should be 8 bytes");
        assert!(!span_id.iter().all(|&b| b == 0), "span_id should not be all zeros");
    }

    #[test]
    fn test_create_root_context() {
        let trace_id = vec![1u8; 16];
        let span_id = vec![2u8; 8];
        let context = create_root_context(&trace_id, &span_id, 0x01);

        assert_eq!(context.trace_id, trace_id);
        assert_eq!(context.span_id, span_id);
        assert_eq!(context.trace_flags, 0x01);
        assert!(!context.is_remote);
    }

    #[test]
    fn test_create_child_context() {
        let parent_trace_id = vec![1u8; 16];
        let parent_span_id = vec![2u8; 8];
        let parent = create_root_context(&parent_trace_id, &parent_span_id, 0x01);

        let child_span_id = vec![3u8; 8];
        let child = create_child_context(&parent, &child_span_id);

        assert_eq!(child.trace_id, parent.trace_id, "Child should inherit parent trace_id");
        assert_eq!(child.span_id, child_span_id);
        assert_eq!(child.trace_flags, parent.trace_flags);
    }

    #[test]
    fn test_is_valid_context() {
        let valid = create_root_context(&vec![1u8; 16], &vec![2u8; 8], 0x01);
        assert!(is_valid_context(&valid));

        let invalid_trace_id = create_root_context(&vec![0u8; 16], &vec![2u8; 8], 0x01);
        assert!(!is_valid_context(&invalid_trace_id));

        let invalid_span_id = create_root_context(&vec![1u8; 16], &vec![0u8; 8], 0x01);
        assert!(!is_valid_context(&invalid_span_id));

        let wrong_trace_length = SpanContext {
            trace_id: vec![1u8; 15],
            span_id: vec![2u8; 8],
            trace_flags: 0x01,
            trace_state: String::new(),
            is_remote: false,
        };
        assert!(!is_valid_context(&wrong_trace_length));
    }

    #[test]
    fn test_is_sampled() {
        let sampled = create_root_context(&vec![1u8; 16], &vec![2u8; 8], 0x01);
        assert!(is_sampled(&sampled));

        let not_sampled = create_root_context(&vec![1u8; 16], &vec![2u8; 8], 0x00);
        assert!(!is_sampled(&not_sampled));
    }

    #[test]
    fn test_set_sampled() {
        let context = create_root_context(&vec![1u8; 16], &vec![2u8; 8], 0x00);
        assert!(!is_sampled(&context));

        let sampled_context = set_sampled(&context, true);
        assert!(is_sampled(&sampled_context));

        let unsampled_context = set_sampled(&sampled_context, false);
        assert!(!is_sampled(&unsampled_context));
    }

    #[test]
    fn test_parse_traceparent_valid() {
        let traceparent = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let result = parse_traceparent_internal(traceparent);
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(context.trace_id.len(), 16);
        assert_eq!(context.span_id.len(), 8);
        assert_eq!(context.trace_flags, 0x01);
    }

    #[test]
    fn test_parse_traceparent_invalid_version() {
        let traceparent = "01-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let result = parse_traceparent_internal(traceparent);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported traceparent version"));
    }

    #[test]
    fn test_parse_traceparent_invalid_parts() {
        let traceparent = "00-0af7651916cd43dd8448eb211c80319c-01";
        let result = parse_traceparent_internal(traceparent);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expected 4 parts"));
    }

    #[test]
    fn test_parse_traceparent_all_zeros_trace_id() {
        let traceparent = "00-00000000000000000000000000000000-b7ad6b7169203331-01";
        let result = parse_traceparent_internal(traceparent);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("trace_id cannot be all zeros"));
    }

    #[test]
    fn test_parse_traceparent_all_zeros_span_id() {
        let traceparent = "00-0af7651916cd43dd8448eb211c80319c-0000000000000000-01";
        let result = parse_traceparent_internal(traceparent);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("span_id cannot be all zeros"));
    }

    #[test]
    fn test_format_traceparent() {
        let context = SpanContext {
            trace_id: hex::decode("0af7651916cd43dd8448eb211c80319c").unwrap(),
            span_id: hex::decode("b7ad6b7169203331").unwrap(),
            trace_flags: 0x01,
            trace_state: String::new(),
            is_remote: false,
        };

        let formatted = format_traceparent_internal(&context);
        assert_eq!(formatted, "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01");
    }

    #[test]
    fn test_traceparent_roundtrip() {
        let original = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let parsed = parse_traceparent_internal(original).unwrap();
        let formatted = format_traceparent_internal(&parsed);
        assert_eq!(formatted, original);
    }

    #[test]
    fn test_validate_traceparent() {
        assert!(validate_traceparent("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string()));
        assert!(!validate_traceparent("invalid".to_string()));
        assert!(!validate_traceparent("01-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string()));
    }

    #[test]
    fn test_parse_tracestate() {
        assert!(parse_tracestate("".to_string()).is_ok());
        assert!(parse_tracestate("vendor1=value1".to_string()).is_ok());
        assert!(parse_tracestate("vendor1=value1,vendor2=value2".to_string()).is_ok());
        assert!(parse_tracestate("invalid".to_string()).is_err());
    }

    #[test]
    fn test_inject_context() {
        let context = SpanContext {
            trace_id: hex::decode("0af7651916cd43dd8448eb211c80319c").unwrap(),
            span_id: hex::decode("b7ad6b7169203331").unwrap(),
            trace_flags: 0x01,
            trace_state: "vendor=value".to_string(),
            is_remote: false,
        };

        let carriers = inject_context(&context);
        assert_eq!(carriers.len(), 2);
        assert_eq!(carriers[0].key, "traceparent");
        assert_eq!(carriers[0].value, "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01");
        assert_eq!(carriers[1].key, "tracestate");
        assert_eq!(carriers[1].value, "vendor=value");
    }

    #[test]
    fn test_extract_context() {
        let carriers = vec![
            ContextCarrier {
                key: "traceparent".to_string(),
                value: "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
            },
            ContextCarrier {
                key: "tracestate".to_string(),
                value: "vendor=value".to_string(),
            },
        ];

        let result = extract_context(carriers);
        match result {
            ContextResult::Success(context) => {
                assert_eq!(context.trace_id.len(), 16);
                assert_eq!(context.span_id.len(), 8);
                assert_eq!(context.trace_flags, 0x01);
                assert_eq!(context.trace_state, "vendor=value");
                assert!(context.is_remote);
            }
            ContextResult::Invalid(e) => panic!("Expected Success, got Invalid: {}", e),
            ContextResult::NotFound => panic!("Expected Success, got NotFound"),
        }
    }

    #[test]
    fn test_active_context_management() {
        // Clear any previous state
        clear_active_context();

        // Should be None initially
        assert!(get_active_context().is_none());

        // Set a context
        let context = create_root_context(&vec![1u8; 16], &vec![2u8; 8], 0x01);
        set_active_context(context.clone());

        // Should retrieve the same context
        let retrieved = get_active_context();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.trace_id, context.trace_id);
        assert_eq!(retrieved.span_id, context.span_id);

        // Clear should reset to None
        clear_active_context();
        assert!(get_active_context().is_none());
    }
}
