//! Initialize result writer component for the Model Context Protocol (MCP).
//!
//! This component exports the initialize-writer interface for writing
//! initialization results to output streams in the JSON-RPC format.

#[rustfmt::skip]
#[allow(clippy::all)]
#[allow(dead_code)]
#[allow(unused_imports)]
#[allow(non_snake_case)]
mod bindings;

use bindings::exports::wasmcp::mcp::initialize_result::{
    Guest, Id, InitializeResult, OutputStream, ProtocolVersion, ServerCapabilities, StreamError,
};

pub struct Component;

impl Guest for Component {
    fn write(id: Id, output: OutputStream, result: InitializeResult) -> Result<(), StreamError> {
        // Build the complete response as a single string
        // This is more efficient for small, bounded payloads like initialize results
        let mut response = String::with_capacity(1024); // Pre-allocate reasonable capacity

        // Opening JSON-RPC envelope
        response.push_str(r#"{"jsonrpc":"2.0","id":"#);
        match id {
            Id::Number(n) => response.push_str(&n.to_string()),
            Id::String(s) => {
                response
                    .push_str(&serde_json::to_string(&s).unwrap_or_else(|_| r#""""#.to_string()));
            }
        }
        response.push_str(r#","result":{"#);

        // Protocol version
        response.push_str(r#""protocolVersion":""#);
        response.push_str(match result.protocol_version {
            ProtocolVersion::V20250618 => "2025-06-18",
            ProtocolVersion::V20250326 => "2025-03-26",
            ProtocolVersion::V20241105 => "2024-11-05",
        });
        response.push_str(r#"","#);

        // Capabilities
        response.push_str(r#""capabilities":{"#);
        let mut first_capability = true;
        let capabilities_flags = [
            (ServerCapabilities::COMPLETIONS, "completions"),
            (ServerCapabilities::PROMPTS, "prompts"),
            (ServerCapabilities::RESOURCES, "resources"),
            (ServerCapabilities::TOOLS, "tools"),
            (ServerCapabilities::EXPERIMENTAL, "experimental"),
        ];

        for (flag, name) in capabilities_flags {
            if result.capabilities.contains(flag) {
                if !first_capability {
                    response.push(',');
                }
                response.push('"');
                response.push_str(name);
                response.push_str(r#"":{}"#);
                first_capability = false;
            }
        }
        response.push_str("},");

        // Server info
        response.push_str(r#""serverInfo":{"name":""#);
        push_escaped_string(&mut response, &result.server_info.name);
        response.push_str(r#"","version":""#);
        push_escaped_string(&mut response, &result.server_info.version);
        response.push('"');

        if let Some(title) = &result.server_info.title {
            response.push_str(r#","title":""#);
            push_escaped_string(&mut response, title);
            response.push('"');
        }
        response.push('}');

        // Optional fields
        if let Some(options) = &result.options {
            if let Some(instructions) = &options.instructions {
                response.push_str(r#","instructions":""#);
                push_escaped_string(&mut response, instructions);
                response.push('"');
            }
            if let Some(meta) = &options.meta {
                response.push_str(r#","meta":{"#);
                let mut first_meta = true;
                for (key, value) in meta {
                    if !first_meta {
                        response.push(',');
                    }
                    response.push('"');
                    push_escaped_string(&mut response, key);
                    response.push_str(r#"":""#);
                    push_escaped_string(&mut response, value);
                    response.push('"');
                    first_meta = false;
                }
                response.push('}');
            }
        }

        // Close the result object and JSON-RPC envelope
        response.push_str("}}");

        // Add newline for MCP stdio protocol (newline-delimited JSON)
        response.push('\n');

        // Write the complete response efficiently
        write_bytes_to_stream(&output, response.as_bytes())
    }
}

/// Efficiently write bytes to a WASI output stream
/// Handles backpressure properly according to WASI I/O semantics
fn write_bytes_to_stream(output: &OutputStream, bytes: &[u8]) -> Result<(), StreamError> {
    let mut offset = 0;

    while offset < bytes.len() {
        // Check available capacity
        let capacity = output.check_write().map_err(|_| StreamError::Closed)? as usize;

        if capacity == 0 {
            // No capacity available, use blocking write for remainder
            output
                .blocking_write_and_flush(&bytes[offset..])
                .map_err(|_| StreamError::Closed)?;
            return Ok(());
        }

        // Write what we can without blocking
        let chunk_size = capacity.min(bytes.len() - offset);
        output
            .write(&bytes[offset..offset + chunk_size])
            .map_err(|_| StreamError::Closed)?;
        offset += chunk_size;
    }

    // Flush once at the end
    output.flush().map_err(|_| StreamError::Closed)
}

/// Push a string to the response buffer with proper JSON escaping
fn push_escaped_string(response: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '"' => response.push_str("\\\""),
            '\\' => response.push_str("\\\\"),
            '\n' => response.push_str("\\n"),
            '\r' => response.push_str("\\r"),
            '\t' => response.push_str("\\t"),
            '\x08' => response.push_str("\\b"),
            '\x0C' => response.push_str("\\f"),
            c if c.is_control() => {
                response.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => response.push(c),
        }
    }
}

bindings::export!(Component with_types_in bindings);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_exists() {
        // Basic test to ensure the component compiles
        let _component = Component;
    }
}
