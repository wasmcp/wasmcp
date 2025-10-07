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
        let values_json = build_completion_values_array(&values);

        let mut result = JsonObjectBuilder::new();
        result.add_field("completions", &values_json);

        let response = build_jsonrpc_response(&id, &result.build());
        write_sse_message(&out, &response)
    }

    type Writer = CompleteWriterResource;

    fn open(
        id: Id,
        out: OutputStream,
    ) -> Result<complete_writer::Writer, StreamError> {
        Ok(complete_writer::Writer::new(CompleteWriterResource {
            state: RefCell::new(CompleteWriterState {
                id,
                out,
                values: Vec::new(),
            }),
        }))
    }
}

pub struct CompleteWriterResource {
    state: RefCell<CompleteWriterState>,
}

struct CompleteWriterState {
    id: Id,
    out: OutputStream,
    values: Vec<String>,
}

impl complete_writer::GuestWriter for CompleteWriterResource {
    fn write(&self, value: String) -> Result<(), StreamError> {
        let mut state = self.state.borrow_mut();
        state.values.push(value);
        Ok(())
    }

    fn close(&self, options: Option<complete_writer::Options>) -> Result<(), StreamError> {
        let state = self.state.borrow();
        let values_json = build_completion_values_array(&state.values);

        let mut result = JsonObjectBuilder::new();
        result.add_field("completions", &values_json);

        // Add optional hasMore field
        if let Some(opts) = options {
            if let Some(has_more) = opts.has_more {
                result.add_bool("hasMore", has_more);
            }
        }

        let response = build_jsonrpc_response(&state.id, &result.build());
        write_sse_message(&state.out, &response)
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