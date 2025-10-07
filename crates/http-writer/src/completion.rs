//! Completion writer implementation for HTTP/SSE transport.
//!
//! Handles serialization of auto-completion suggestions for prompts and commands.

use crate::bindings::exports::wasmcp::mcp::complete_writer;
use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use crate::bindings::wasmcp::mcp::protocol::Id;
use crate::utils::{
    build_jsonrpc_response, escape_json_string, write_sse_message,
    JsonObjectBuilder,
};
use std::cell::RefCell;

pub struct CompleteWriter;

impl complete_writer::Guest for CompleteWriter {
    fn write(
        id: Id,
        out: OutputStream,
        values: Vec<String>,
    ) -> Result<(), StreamError> {
        // One-shot: Build complete response and send
        let values_json = build_completion_values_array(&values);

        let mut result = JsonObjectBuilder::new();
        result.add_field("completions", &values_json);

        let response = build_jsonrpc_response(&id, &result.build());
        write_sse_message(&out, &response)?;

        // Flush to ensure delivery
        out.flush()?;
        Ok(())
    }

    type Writer = CompleteWriterResource;

    fn open(
        id: Id,
        out: OutputStream,
    ) -> Result<complete_writer::Writer, StreamError> {
        // Start the JSON-RPC response and completions array
        let id_str = crate::utils::format_id(&id);
        let header = format!(
            r#"{{"jsonrpc":"2.0","id":{id_str},"result":{{"completions":["#
        );

        // Write the opening of the response
        write_sse_message(&out, &header)?;

        Ok(complete_writer::Writer::new(CompleteWriterResource {
            state: RefCell::new(CompleteWriterState {
                out,
                first_item: true,
                closed: false,
            }),
        }))
    }
}

pub struct CompleteWriterResource {
    state: RefCell<CompleteWriterState>,
}

struct CompleteWriterState {
    out: OutputStream,
    first_item: bool,
    closed: bool,
}

impl complete_writer::GuestWriter for CompleteWriterResource {
    fn write(&self, value: String) -> Result<(), StreamError> {
        let mut state = self.state.borrow_mut();

        if state.closed {
            return Err(StreamError::Closed);
        }

        // Add comma separator if not first item
        let mut output = String::new();
        if !state.first_item {
            output.push(',');
        } else {
            state.first_item = false;
        }

        // Escape and append this single completion value
        output.push('"');
        output.push_str(&escape_json_string(&value));
        output.push('"');

        // Write immediately - true streaming!
        write_sse_message(&state.out, &output)?;

        Ok(())
    }

    fn close(&self, options: Option<complete_writer::Options>) -> Result<(), StreamError> {
        let mut state = self.state.borrow_mut();

        if state.closed {
            return Err(StreamError::Closed);
        }

        // Close the completions array and add optional fields
        let mut closing = String::from("]");

        // Add optional hasMore field
        if let Some(opts) = options {
            if let Some(has_more) = opts.has_more {
                closing.push_str(r#","hasMore":"#);
                closing.push_str(if has_more { "true" } else { "false" });
            }
        }

        // Close the result object and JSON-RPC response
        closing.push_str("}}");

        // Write the closing
        write_sse_message(&state.out, &closing)?;

        // Final flush to ensure all data is sent
        state.out.flush()?;
        state.closed = true;

        Ok(())
    }
}

/// Build a JSON array of completion values.
fn build_completion_values_array(values: &[String]) -> String {
    if values.is_empty() {
        return "[]".to_string();
    }

    let escaped_values: Vec<String> = values
        .iter()
        .map(|v| format!(r#""{}""#, escape_json_string(v)))
        .collect();

    format!("[{}]", escaped_values.join(","))
}