//! Prompts writer implementations for HTTP/SSE transport.
//!
//! Handles serialization of prompt-related responses including:
//! - Prompt listings with argument schemas
//! - Prompt message generation with role and content

use crate::bindings::exports::wasmcp::mcp::{list_prompts_writer, get_prompt_writer};
use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use crate::bindings::wasmcp::mcp::protocol::{Id, Role};
use crate::utils::{
    build_content_block_json, build_jsonrpc_response, build_meta_object,
    write_sse_message, JsonObjectBuilder,
};
use std::cell::RefCell;

pub struct ListPromptsWriter;
pub struct GetPromptWriter;

// ===== List Prompts Writer =====

impl list_prompts_writer::Guest for ListPromptsWriter {
    fn write(
        id: Id,
        out: OutputStream,
        prompts: Vec<list_prompts_writer::Prompt>,
    ) -> Result<(), StreamError> {
        let prompts_json = build_prompts_array(&prompts);

        let mut result = JsonObjectBuilder::new();
        result.add_field("prompts", &prompts_json);

        let response = build_jsonrpc_response(&id, &result.build());
        write_sse_message(&out, &response)
    }

    type Writer = ListPromptsWriterResource;

    fn open(
        id: Id,
        out: OutputStream,
    ) -> Result<list_prompts_writer::Writer, StreamError> {
        Ok(list_prompts_writer::Writer::new(ListPromptsWriterResource {
            state: RefCell::new(PromptsWriterState {
                id,
                out,
                prompts: Vec::new(),
            }),
        }))
    }
}

pub struct ListPromptsWriterResource {
    state: RefCell<PromptsWriterState>,
}

struct PromptsWriterState {
    id: Id,
    out: OutputStream,
    prompts: Vec<list_prompts_writer::Prompt>,
}

impl list_prompts_writer::GuestWriter for ListPromptsWriterResource {
    fn write(&self, prompt: list_prompts_writer::Prompt) -> Result<(), StreamError> {
        let mut state = self.state.borrow_mut();
        state.prompts.push(prompt);
        Ok(())
    }

    fn close(&self, options: Option<list_prompts_writer::Options>) -> Result<(), StreamError> {
        let state = self.state.borrow();
        let prompts_json = build_prompts_array(&state.prompts);

        let mut result = JsonObjectBuilder::new();
        result.add_field("prompts", &prompts_json);

        if let Some(opts) = options {
            if let Some(meta) = opts.meta {
                if !meta.is_empty() {
                    result.add_field("_meta", &build_meta_object(&meta));
                }
            }
            if let Some(cursor) = &opts.next_cursor {
                result.add_string("nextCursor", cursor);
            }
        }

        let response = build_jsonrpc_response(&state.id, &result.build());
        write_sse_message(&state.out, &response)
    }
}

// ===== Get Prompt Writer =====

impl get_prompt_writer::Guest for GetPromptWriter {
    fn write(
        id: Id,
        out: OutputStream,
        messages: Vec<get_prompt_writer::PromptMessage>,
    ) -> Result<(), StreamError> {
        let messages_json = build_prompt_messages_array(&messages);

        let mut result = JsonObjectBuilder::new();
        result.add_field("messages", &messages_json);

        let response = build_jsonrpc_response(&id, &result.build());
        write_sse_message(&out, &response)
    }

    type Writer = GetPromptWriterResource;

    fn open(
        id: Id,
        out: OutputStream,
    ) -> Result<get_prompt_writer::Writer, StreamError> {
        Ok(get_prompt_writer::Writer::new(GetPromptWriterResource {
            state: RefCell::new(GetPromptWriterState {
                id,
                out,
                messages: Vec::new(),
            }),
        }))
    }
}

pub struct GetPromptWriterResource {
    state: RefCell<GetPromptWriterState>,
}

struct GetPromptWriterState {
    id: Id,
    out: OutputStream,
    messages: Vec<get_prompt_writer::PromptMessage>,
}

impl get_prompt_writer::GuestWriter for GetPromptWriterResource {
    fn write(&self, message: get_prompt_writer::PromptMessage) -> Result<(), StreamError> {
        let mut state = self.state.borrow_mut();
        state.messages.push(message);
        Ok(())
    }

    fn close(&self, options: Option<get_prompt_writer::Options>) -> Result<(), StreamError> {
        let state = self.state.borrow();
        let messages_json = build_prompt_messages_array(&state.messages);

        let mut result = JsonObjectBuilder::new();
        result.add_field("messages", &messages_json);

        if let Some(opts) = options {
            if let Some(meta) = opts.meta {
                if !meta.is_empty() {
                    result.add_field("_meta", &build_meta_object(&meta));
                }
            }
            if let Some(desc) = opts.description {
                result.add_string("description", &desc);
            }
        }

        let response = build_jsonrpc_response(&state.id, &result.build());
        write_sse_message(&state.out, &response)
    }
}

// ===== Helper Functions =====

/// Build a JSON array of prompts.
fn build_prompts_array(prompts: &[list_prompts_writer::Prompt]) -> String {
    if prompts.is_empty() {
        return "[]".to_string();
    }

    let prompt_objects: Vec<String> = prompts.iter().map(|prompt| {
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
    }).collect();

    format!("[{}]", prompt_objects.join(","))
}

/// Build a JSON array of prompt arguments.
fn build_prompt_arguments_array(arguments: &[list_prompts_writer::PromptArgument]) -> String {
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

/// Build a JSON array of prompt messages.
fn build_prompt_messages_array(messages: &[get_prompt_writer::PromptMessage]) -> String {
    if messages.is_empty() {
        return "[]".to_string();
    }

    let message_objects: Vec<String> = messages.iter().map(|message| {
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
    }).collect();

    format!("[{}]", message_objects.join(","))
}

