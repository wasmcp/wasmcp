//! Prompts writer implementations for HTTP/SSE transport.
//!
//! Handles serialization of prompt-related responses including:
//! - Prompt listings with argument schemas
//! - Prompt message generation with role and content

use crate::bindings::exports::wasmcp::mcp::{list_prompts_writer, get_prompt_writer};
use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use crate::bindings::wasmcp::mcp::protocol::{
    Id, Role, NextCursorOptions, DescriptionOptions, PromptArgument,
};
use crate::utils::{
    build_content_block_json, build_jsonrpc_response, build_meta_object,
    write_message, JsonObjectBuilder,
};
use std::cell::RefCell;

pub struct ListPromptsWriter;
pub struct GetPromptWriter;

// ===== List Prompts Writer =====

impl list_prompts_writer::Guest for ListPromptsWriter {
    fn send(
        id: Id,
        out: OutputStream,
        prompts: Vec<list_prompts_writer::Prompt>,
    ) -> Result<(), StreamError> {
        // One-shot: Build complete response and send
        let prompts_json = build_prompts_array(&prompts);

        let mut result = JsonObjectBuilder::new();
        result.add_field("prompts", &prompts_json);

        let response = build_jsonrpc_response(&id, &result.build());
        write_message(&out, &response)?;

        // Flush to ensure delivery
        out.flush()?;
        Ok(())
    }

    type Writer = ListPromptsWriterResource;

    fn open(
        id: Id,
        out: OutputStream,
    ) -> Result<list_prompts_writer::Writer, StreamError> {
        // Start the JSON-RPC response and prompts array
        let id_str = crate::utils::format_id(&id);
        let header = format!(
            r#"{{"jsonrpc":"2.0","id":{id_str},"result":{{"prompts":["#
        );

        // Write the opening of the response
        write_message(&out, &header)?;

        Ok(list_prompts_writer::Writer::new(ListPromptsWriterResource {
            state: RefCell::new(PromptsWriterState {
                out,
                first_item: true,
                closed: false,
            }),
        }))
    }
}

pub struct ListPromptsWriterResource {
    state: RefCell<PromptsWriterState>,
}

struct PromptsWriterState {
    out: OutputStream,
    first_item: bool,
    closed: bool,
}

impl list_prompts_writer::GuestWriter for ListPromptsWriterResource {
    fn write(&self, prompt: list_prompts_writer::Prompt) -> Result<(), StreamError> {
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

        // Build and append this single prompt
        let prompt_json = build_single_prompt(&prompt);
        output.push_str(&prompt_json);

        // Write immediately - true streaming!
        write_message(&state.out, &output)?;

        Ok(())
    }

    fn close(&self, options: Option<NextCursorOptions>) -> Result<(), StreamError> {
        let mut state = self.state.borrow_mut();

        if state.closed {
            return Err(StreamError::Closed);
        }

        // Close the prompts array and add optional fields
        let mut closing = String::from("]");

        if let Some(opts) = options {
            if let Some(meta) = opts.meta {
                if !meta.is_empty() {
                    closing.push_str(r#","_meta":"#);
                    closing.push_str(&build_meta_object(&meta));
                }
            }
            if let Some(cursor) = opts.next_cursor.as_ref() {
                closing.push_str(r#","nextCursor":""#);
                closing.push_str(&crate::utils::escape_json_string(cursor));
                closing.push('"');
            }
        }

        // Close the result object and JSON-RPC response
        closing.push_str("}}");

        // Write the closing
        write_message(&state.out, &closing)?;

        // Final flush to ensure all data is sent
        state.out.flush()?;
        state.closed = true;

        Ok(())
    }
}

// ===== Get Prompt Writer =====

impl get_prompt_writer::Guest for GetPromptWriter {
    fn send(
        id: Id,
        out: OutputStream,
        messages: Vec<get_prompt_writer::PromptMessage>,
    ) -> Result<(), StreamError> {
        // One-shot: Build complete response and send
        let messages_json = build_prompt_messages_array(&messages);

        let mut result = JsonObjectBuilder::new();
        result.add_field("messages", &messages_json);

        let response = build_jsonrpc_response(&id, &result.build());
        write_message(&out, &response)?;

        // Flush to ensure delivery
        out.flush()?;
        Ok(())
    }

    type Writer = GetPromptWriterResource;

    fn open(
        id: Id,
        out: OutputStream,
    ) -> Result<get_prompt_writer::Writer, StreamError> {
        // Start the JSON-RPC response and messages array
        let id_str = crate::utils::format_id(&id);
        let header = format!(
            r#"{{"jsonrpc":"2.0","id":{id_str},"result":{{"messages":["#
        );

        // Write the opening of the response
        write_message(&out, &header)?;

        Ok(get_prompt_writer::Writer::new(GetPromptWriterResource {
            state: RefCell::new(GetPromptWriterState {
                out,
                first_item: true,
                closed: false,
            }),
        }))
    }
}

pub struct GetPromptWriterResource {
    state: RefCell<GetPromptWriterState>,
}

struct GetPromptWriterState {
    out: OutputStream,
    first_item: bool,
    closed: bool,
}

impl get_prompt_writer::GuestWriter for GetPromptWriterResource {
    fn write(&self, message: get_prompt_writer::PromptMessage) -> Result<(), StreamError> {
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

        // Build and append this single message
        let message_json = build_single_prompt_message(&message);
        output.push_str(&message_json);

        // Write immediately - true streaming!
        write_message(&state.out, &output)?;

        Ok(())
    }

    fn close(&self, options: Option<DescriptionOptions>) -> Result<(), StreamError> {
        let mut state = self.state.borrow_mut();

        if state.closed {
            return Err(StreamError::Closed);
        }

        // Close the messages array and add optional fields
        let mut closing = String::from("]");

        if let Some(opts) = options {
            if let Some(meta) = opts.meta {
                if !meta.is_empty() {
                    closing.push_str(r#","_meta":"#);
                    closing.push_str(&build_meta_object(&meta));
                }
            }
            if let Some(desc) = opts.description {
                closing.push_str(r#","description":""#);
                closing.push_str(&crate::utils::escape_json_string(&desc));
                closing.push('"');
            }
        }

        // Close the result object and JSON-RPC response
        closing.push_str("}}");

        // Write the closing
        write_message(&state.out, &closing)?;

        // Final flush to ensure all data is sent
        state.out.flush()?;
        state.closed = true;

        Ok(())
    }
}

// ===== Helper Functions =====

/// Build a JSON array of prompts (for one-shot send).
fn build_prompts_array(prompts: &[list_prompts_writer::Prompt]) -> String {
    if prompts.is_empty() {
        return "[]".to_string();
    }

    let prompt_objects: Vec<String> = prompts.iter()
        .map(build_single_prompt)
        .collect();

    format!("[{}]", prompt_objects.join(","))
}

/// Build JSON for a single prompt.
fn build_single_prompt(prompt: &list_prompts_writer::Prompt) -> String {
    let mut obj = JsonObjectBuilder::new();
    obj.add_string("name", &prompt.name);

    if let Some(opts) = &prompt.options {
        obj.add_optional_string("description", opts.description.as_deref());
        obj.add_optional_string("title", opts.title.as_deref());

        if let Some(arguments) = &opts.arguments {
            obj.add_field("arguments", &build_prompt_arguments_array(arguments));
        }

        if let Some(meta) = &opts.meta {
            if !meta.is_empty() {
                obj.add_field("_meta", &build_meta_object(meta));
            }
        }
    }

    obj.build()
}

/// Build a JSON array of prompt arguments.
fn build_prompt_arguments_array(arguments: &[PromptArgument]) -> String {
    if arguments.is_empty() {
        return "[]".to_string();
    }

    let arg_objects: Vec<String> = arguments.iter().map(|arg| {
        let mut obj = JsonObjectBuilder::new();
        obj.add_string("name", &arg.name);
        obj.add_optional_string("description", arg.description.as_deref());
        obj.add_optional_string("title", arg.title.as_deref());

        if let Some(required) = arg.required {
            obj.add_bool("required", required);
        }

        obj.build()
    }).collect();

    format!("[{}]", arg_objects.join(","))
}

/// Build a JSON array of prompt messages (for one-shot send).
fn build_prompt_messages_array(messages: &[get_prompt_writer::PromptMessage]) -> String {
    if messages.is_empty() {
        return "[]".to_string();
    }

    let message_objects: Vec<String> = messages.iter()
        .map(build_single_prompt_message)
        .collect();

    format!("[{}]", message_objects.join(","))
}

/// Build JSON for a single prompt message.
fn build_single_prompt_message(message: &get_prompt_writer::PromptMessage) -> String {
    let mut obj = JsonObjectBuilder::new();

    // Add role field
    let role_str = match message.role {
        Role::User => "user",
        Role::Assistant => "assistant",
    };
    obj.add_string("role", role_str);

    // Add content - need to build the content block JSON
    obj.add_field("content", &build_content_block_json(&message.content));

    obj.build()
}

