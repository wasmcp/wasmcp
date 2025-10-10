//! Completions response writers
//!
//! Implements serialization for auto-completion suggestions for prompts and commands.

use crate::bindings::exports::wasmcp::mcp::completions_response::{
    self, CompletionResultOptions, Completions,
};
use crate::bindings::wasmcp::mcp::output::{
    finish_message, start_message, write_message_contents, IoError,
};
use crate::bindings::wasmcp::mcp::protocol::Id;
use crate::utils::{escape_json_string, JsonObjectBuilder};
use std::cell::RefCell;

// === Simple Response Function ===

/// Write a completion/complete response with complete values list.
pub fn write_completions(id: Id, completions: Completions) -> Result<(), IoError> {
    let values_json = build_completion_values_array(&completions.values);

    let mut completion_obj = JsonObjectBuilder::new();
    completion_obj.add_raw_json("values", &values_json);

    // Add optional fields if provided
    if let Some(opts) = completions.options {
        if let Some(total) = opts.total {
            completion_obj.add_number("total", total as f64);
        }
        if let Some(has_more) = opts.has_more {
            completion_obj.add_bool("hasMore", has_more);
        }
    }

    // Wrap in the result with "completion" (singular) field per MCP schema
    let mut result = JsonObjectBuilder::new();
    result.add_raw_json("completion", &completion_obj.build());

    let response = build_json_rpc_response(&id, &result.build());

    start_message()?;
    write_message_contents(&response.into_bytes())?;
    finish_message()
}

// === Streaming Completions Writer Resource ===

pub struct CompletionsWriter {
    state: RefCell<CompletionsWriterState>,
}

struct CompletionsWriterState {
    first_item: bool,
}

impl crate::bindings::exports::wasmcp::mcp::completions_response::GuestCompletionsWriter
    for CompletionsWriter
{
    fn start(id: Id) -> Result<completions_response::CompletionsWriter, IoError> {
        let id_str = format_id(&id);
        // Note: nested "completion" object with "values" array per MCP schema
        let header =
            format!(r#"{{"jsonrpc":"2.0","id":{id_str},"result":{{"completion":{{"values":["#);

        start_message()?;
        write_message_contents(&header.into_bytes())?;

        Ok(completions_response::CompletionsWriter::new(
            CompletionsWriter {
                state: RefCell::new(CompletionsWriterState { first_item: true }),
            },
        ))
    }

    fn write(&self, value: String) -> Result<(), IoError> {
        let mut state = self.state.borrow_mut();

        let mut output_str = String::new();
        if !state.first_item {
            output_str.push(',');
        } else {
            state.first_item = false;
        }

        // Escape and append this single completion value
        output_str.push('"');
        output_str.push_str(&escape_json_string(&value));
        output_str.push('"');

        write_message_contents(&output_str.into_bytes())
    }

    fn finish(
        _this: completions_response::CompletionsWriter,
        options: Option<CompletionResultOptions>,
    ) -> Result<(), IoError> {
        // Close the values array
        let mut closing = String::from("]");

        // Add optional fields within the completion object
        if let Some(opts) = options {
            if let Some(total) = opts.total {
                closing.push_str(&format!(r#","total":{}"#, total));
            }
            if let Some(has_more) = opts.has_more {
                closing.push_str(&format!(r#","hasMore":{}"#, has_more));
            }
        }

        // Close the completion object, result object, and JSON-RPC response
        closing.push_str("}}}");

        write_message_contents(&closing.into_bytes())?;
        finish_message()
    }
}

// === Helper Functions ===

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

/// Build a JSON-RPC 2.0 response.
fn build_json_rpc_response(id: &Id, result: &str) -> String {
    let id_str = format_id(id);
    format!(r#"{{"jsonrpc":"2.0","id":{},"result":{}}}"#, id_str, result)
}

/// Format an ID value as JSON.
fn format_id(id: &Id) -> String {
    match id {
        Id::Number(n) => n.to_string(),
        Id::String(s) => format!(r#""{}""#, escape_json_string(s)),
    }
}
