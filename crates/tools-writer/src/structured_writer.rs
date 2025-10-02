//! Structured writer for tools/call results
//!
//! This implements the tools-call-structured interface which only has
//! a one-shot write() function - NO streaming.

use crate::bindings::exports::wasmcp::mcp::tools_call_structured::{Id, Options};
use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use crate::helpers::{meta_to_option_json, write_to_stream};
use serde_json::json;

/// Write a structured tools/call result to the output stream.
///
/// This is a one-shot operation with no streaming support.
pub fn write_structured(
    id: Id,
    output: OutputStream,
    structured: String,
    options: Option<Options>,
) -> Result<(), StreamError> {
    // Parse structured content as JSON
    let content_value = serde_json::from_str(&structured)
        .unwrap_or_else(|_| json!({"type": "text", "text": structured}));

    // Build the JSON-RPC response
    let mut response = json!({
        "jsonrpc": "2.0",
        "id": match &id {
            Id::Number(n) => json!(n),
            Id::String(s) => json!(s),
        },
        "result": {
            "content": [content_value]
        }
    });

    // Add optional fields
    if let Some(opts) = options {
        if opts.is_error {
            response["result"]["isError"] = json!(true);
        }
        if let Some(meta) = meta_to_option_json(&opts.meta) {
            response["result"]["_meta"] = meta;
        }
    }

    // Write to stream with newline for stdio protocol
    let response_str = serde_json::to_string(&response).map_err(|_| StreamError::Closed)?;
    write_to_stream(&output, response_str.as_bytes())?;
    write_to_stream(&output, b"\n")?;
    output.flush().map_err(|_| StreamError::Closed)?;

    Ok(())
}

// Implement the Guest trait for Component
impl crate::bindings::exports::wasmcp::mcp::tools_call_structured::Guest for crate::Component {
    fn write(
        id: Id,
        output: OutputStream,
        structured: String,
        options: Option<Options>,
    ) -> Result<(), StreamError> {
        write_structured(id, output, structured, options)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_json_parsing() {
        // Test valid JSON
        let valid = r#"{"type": "text", "text": "hello"}"#;
        let parsed: serde_json::Value = serde_json::from_str(valid).unwrap();
        assert_eq!(parsed["type"], "text");

        // Test invalid JSON fallback
        let invalid = "not json";
        let fallback = serde_json::from_str::<serde_json::Value>(invalid)
            .unwrap_or_else(|_| json!({"type": "text", "text": invalid}));
        assert_eq!(fallback["type"], "text");
        assert_eq!(fallback["text"], "not json");
    }
}
