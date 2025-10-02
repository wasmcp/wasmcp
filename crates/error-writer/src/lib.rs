//! Error writer component for the Model Context Protocol (MCP).
//!
//! This component exports the error-result interface for writing
//! JSON-RPC error responses to output streams.

#[allow(warnings)]
mod bindings;

use bindings::exports::wasmcp::mcp::error_result::{
    Guest, Id, McpError, OutputStream, StreamError,
};
use bindings::wasmcp::mcp::error::ErrorCode;

pub struct Component;

impl Guest for Component {
    fn write(id: Id, output: OutputStream, error: McpError) -> Result<(), StreamError> {
        // Map ErrorCode enum to JSON-RPC numeric codes
        let code = match error.code {
            ErrorCode::ParseError => -32700,
            ErrorCode::InvalidRequest => -32600,
            ErrorCode::MethodNotFound => -32601,
            ErrorCode::InvalidParams => -32602,
            ErrorCode::InternalError => -32603,
        };

        // Build JSON-RPC error response
        let mut response = String::with_capacity(256);

        response.push_str(r#"{"jsonrpc":"2.0","id":"#);
        match id {
            Id::Number(n) => response.push_str(&n.to_string()),
            Id::String(s) => {
                response
                    .push_str(&serde_json::to_string(&s).unwrap_or_else(|_| r#""""#.to_string()));
            }
        }

        response.push_str(r#","error":{"code":"#);
        response.push_str(&code.to_string());
        response.push_str(r#","message":""#);
        push_escaped_string(&mut response, &error.message);
        response.push('"');

        // Add optional data field if present
        if let Some(data) = &error.data {
            response.push_str(r#","data":""#);
            push_escaped_string(&mut response, data);
            response.push('"');
        }

        response.push_str("}}");

        // Add newline for MCP stdio protocol (newline-delimited JSON)
        response.push('\n');

        // Write to stream and flush
        write_to_stream(&output, response.as_bytes())
    }
}

/// Write bytes to output stream handling backpressure
fn write_to_stream(output: &OutputStream, bytes: &[u8]) -> Result<(), StreamError> {
    let mut offset = 0;

    while offset < bytes.len() {
        let capacity = output.check_write().map_err(|_| StreamError::Closed)? as usize;

        if capacity == 0 {
            output
                .blocking_write_and_flush(&bytes[offset..])
                .map_err(|_| StreamError::Closed)?;
            return Ok(());
        }

        let chunk_size = capacity.min(bytes.len() - offset);
        output
            .write(&bytes[offset..offset + chunk_size])
            .map_err(|_| StreamError::Closed)?;
        offset += chunk_size;
    }

    output.flush().map_err(|_| StreamError::Closed)
}

/// Push a string with proper JSON escaping
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
        let _component = Component;
    }
}
