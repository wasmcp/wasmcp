//! Prompts response writers
//!
//! Implements serialization for prompt-related MCP responses including:
//! - Prompt listings with argument schemas
//! - Prompt message generation with role and content

use crate::bindings::exports::wasmcp::mcp::prompts_response;
use crate::bindings::wasmcp::mcp::output::{
    finish_message, start_message, write_message_contents, IoError,
};
use crate::bindings::wasmcp::mcp::protocol::{
    DescriptionOptions, Id, NextCursorOptions, Prompt, PromptMessage, Role,
};
use crate::utils::{build_content_block_json, escape_json_string, JsonObjectBuilder};
use std::cell::RefCell;

// === Simple Response Functions ===

/// Write a prompts/list response with complete prompt list.
pub fn write_prompts(id: Id, prompts: Vec<Prompt>) -> Result<(), IoError> {
    let prompts_json = build_prompts_array(&prompts);

    let mut result = JsonObjectBuilder::new();
    result.add_raw_json("prompts", &prompts_json);

    let response = build_json_rpc_response(&id, &result.build());

    start_message()?;
    write_message_contents(&response.into_bytes())?;
    finish_message()
}

/// Write a prompts/get response with prompt messages.
pub fn write_prompt_messages(id: Id, messages: Vec<PromptMessage>) -> Result<(), IoError> {
    let messages_json = build_prompt_messages_array(&messages);

    let mut result = JsonObjectBuilder::new();
    result.add_raw_json("messages", &messages_json);

    let response = build_json_rpc_response(&id, &result.build());

    start_message()?;
    write_message_contents(&response.into_bytes())?;
    finish_message()
}

// === Streaming Prompts Writer Resource ===

pub struct PromptsWriter {
    state: RefCell<PromptsWriterState>,
}

struct PromptsWriterState {
    first_item: bool,
}

impl crate::bindings::exports::wasmcp::mcp::prompts_response::GuestPromptsWriter for PromptsWriter {
    fn start(id: Id) -> Result<prompts_response::PromptsWriter, IoError> {
        let id_str = format_id(&id);
        let header = format!(r#"{{"jsonrpc":"2.0","id":{id_str},"result":{{"prompts":["#);

        start_message()?;
        write_message_contents(&header.into_bytes())?;

        Ok(prompts_response::PromptsWriter::new(PromptsWriter {
            state: RefCell::new(PromptsWriterState { first_item: true }),
        }))
    }

    fn write(&self, prompt: Prompt) -> Result<(), IoError> {
        let mut state = self.state.borrow_mut();

        let mut output_str = String::new();
        if !state.first_item {
            output_str.push(',');
        } else {
            state.first_item = false;
        }

        let prompt_json = build_single_prompt(&prompt);
        output_str.push_str(&prompt_json);

        write_message_contents(&output_str.into_bytes())
    }

    fn finish(
        _this: prompts_response::PromptsWriter,
        options: Option<NextCursorOptions>,
    ) -> Result<(), IoError> {
        let mut closing = String::from("]");

        if let Some(opts) = options {
            if let Some(cursor) = opts.next_cursor {
                closing.push_str(&format!(
                    r#","nextCursor":"{}""#,
                    escape_json_string(&cursor)
                ));
            }
            if let Some(meta) = opts.meta {
                if !meta.is_empty() {
                    let meta_json = build_meta_json(&meta);
                    closing.push_str(&format!(r#","_meta":{}"#, meta_json));
                }
            }
        }

        closing.push_str("}}");

        write_message_contents(&closing.into_bytes())?;
        finish_message()
    }
}

// === Streaming Prompt Messages Writer Resource ===

pub struct PromptMessagesWriter {
    state: RefCell<PromptMessagesWriterState>,
}

struct PromptMessagesWriterState {
    first_item: bool,
}

impl crate::bindings::exports::wasmcp::mcp::prompts_response::GuestPromptMessagesWriter
    for PromptMessagesWriter
{
    fn start(id: Id) -> Result<prompts_response::PromptMessagesWriter, IoError> {
        let id_str = format_id(&id);
        let header = format!(r#"{{"jsonrpc":"2.0","id":{id_str},"result":{{"messages":["#);

        start_message()?;
        write_message_contents(&header.into_bytes())?;

        Ok(prompts_response::PromptMessagesWriter::new(
            PromptMessagesWriter {
                state: RefCell::new(PromptMessagesWriterState { first_item: true }),
            },
        ))
    }

    fn write(&self, message: PromptMessage) -> Result<(), IoError> {
        let mut state = self.state.borrow_mut();

        let mut output_str = String::new();
        if !state.first_item {
            output_str.push(',');
        } else {
            state.first_item = false;
        }

        let message_json = build_single_prompt_message(&message);
        output_str.push_str(&message_json);

        write_message_contents(&output_str.into_bytes())
    }

    fn finish(
        _this: prompts_response::PromptMessagesWriter,
        options: Option<DescriptionOptions>,
    ) -> Result<(), IoError> {
        let mut closing = String::from("]");

        if let Some(opts) = options {
            if let Some(description) = opts.description {
                closing.push_str(&format!(
                    r#","description":"{}""#,
                    escape_json_string(&description)
                ));
            }
            if let Some(meta) = opts.meta {
                if !meta.is_empty() {
                    let meta_json = build_meta_json(&meta);
                    closing.push_str(&format!(r#","_meta":{}"#, meta_json));
                }
            }
        }

        closing.push_str("}}");

        write_message_contents(&closing.into_bytes())?;
        finish_message()
    }
}

// === Helper Functions ===

/// Build a JSON array of prompts.
fn build_prompts_array(prompts: &[Prompt]) -> String {
    if prompts.is_empty() {
        return "[]".to_string();
    }

    let prompt_jsons: Vec<String> = prompts.iter().map(build_single_prompt).collect();
    format!("[{}]", prompt_jsons.join(","))
}

/// Build JSON for a single prompt.
fn build_single_prompt(prompt: &Prompt) -> String {
    let mut obj = JsonObjectBuilder::new();
    obj.add_string("name", &prompt.name);

    if let Some(opts) = &prompt.options {
        if let Some(description) = &opts.description {
            obj.add_string("description", description);
        }
        if let Some(arguments) = &opts.arguments {
            obj.add_raw_json("arguments", &build_prompt_arguments_array(arguments));
        }
        if let Some(meta) = &opts.meta {
            if !meta.is_empty() {
                obj.add_raw_json("_meta", &build_meta_json(meta));
            }
        }
    }

    obj.build()
}

/// Build a JSON array of prompt arguments.
fn build_prompt_arguments_array(
    arguments: &[crate::bindings::wasmcp::mcp::protocol::PromptArgument],
) -> String {
    if arguments.is_empty() {
        return "[]".to_string();
    }

    let arg_jsons: Vec<String> = arguments
        .iter()
        .map(|arg| {
            let mut obj = JsonObjectBuilder::new();
            obj.add_string("name", &arg.name);
            if let Some(description) = &arg.description {
                obj.add_string("description", description);
            }
            if let Some(required) = arg.required {
                obj.add_bool("required", required);
            }
            obj.build()
        })
        .collect();

    format!("[{}]", arg_jsons.join(","))
}

/// Build a JSON array of prompt messages.
fn build_prompt_messages_array(messages: &[PromptMessage]) -> String {
    if messages.is_empty() {
        return "[]".to_string();
    }

    let message_jsons: Vec<String> = messages.iter().map(build_single_prompt_message).collect();
    format!("[{}]", message_jsons.join(","))
}

/// Build JSON for a single prompt message.
fn build_single_prompt_message(message: &PromptMessage) -> String {
    let mut obj = JsonObjectBuilder::new();

    // Add role field
    let role_str = match message.role {
        Role::User => "user",
        Role::Assistant => "assistant",
    };
    obj.add_string("role", role_str);

    // Add content using the utility function for content blocks
    obj.add_raw_json("content", &build_content_block_json(&message.content));

    obj.build()
}

/// Build JSON for metadata.
fn build_meta_json(meta: &[(String, String)]) -> String {
    let mut obj = JsonObjectBuilder::new();
    for (key, value) in meta {
        obj.add_string(key, value);
    }
    obj.build()
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
