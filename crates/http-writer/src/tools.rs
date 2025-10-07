//! Tools writer implementations for HTTP/SSE transport.
//!
//! Handles serialization of tool-related responses including:
//! - Tool listings with schemas and metadata
//! - Tool execution results (both structured and unstructured)
//! - Streaming tool output

use crate::bindings::exports::wasmcp::mcp::{
    list_tools_writer, content_tool_writer, structured_tool_writer,
};
use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use crate::bindings::wasmcp::mcp::protocol::{ContentBlock, Id};
use crate::utils::{
    build_content_block_json, build_jsonrpc_response,
    build_meta_object, write_sse_message, JsonObjectBuilder,
};
use std::cell::RefCell;

pub struct ListToolsWriter;
pub struct ContentToolWriter;
pub struct StructuredToolWriter;

// ===== List Tools Writer =====

impl list_tools_writer::Guest for ListToolsWriter {
    fn send(
        id: Id,
        out: OutputStream,
        tools: Vec<list_tools_writer::Tool>,
    ) -> Result<(), StreamError> {
        let tools_json = build_tools_array(&tools);

        let mut result = JsonObjectBuilder::new();
        result.add_field("tools", &tools_json);

        let response = build_jsonrpc_response(&id, &result.build());
        write_sse_message(&out, &response)
    }

    type Writer = ListToolsWriterResource;

    fn open(
        id: Id,
        out: OutputStream,
    ) -> Result<list_tools_writer::Writer, StreamError> {
        Ok(list_tools_writer::Writer::new(ListToolsWriterResource {
            state: RefCell::new(WriterState {
                id,
                out,
                tools: Vec::new(),
            }),
        }))
    }
}

pub struct ListToolsWriterResource {
    state: RefCell<WriterState<list_tools_writer::Tool>>,
}

struct WriterState<T> {
    id: Id,
    out: OutputStream,
    tools: Vec<T>,
}

impl list_tools_writer::GuestWriter for ListToolsWriterResource {
    fn write(&self, tool: list_tools_writer::Tool) -> Result<(), StreamError> {
        let mut state = self.state.borrow_mut();
        state.tools.push(tool);
        Ok(())
    }

    fn close(&self, options: Option<list_tools_writer::Options>) -> Result<(), StreamError> {
        let state = self.state.borrow();
        let tools_json = build_tools_array(&state.tools);

        let mut result = JsonObjectBuilder::new();
        result.add_field("tools", &tools_json);

        if let Some(opts) = options {
            if let Some(meta) = opts.meta {
                if !meta.is_empty() {
                    result.add_field("_meta", &build_meta_object(&meta));
                }
            }
            if let Some(Some(cursor)) = opts.next_cursor.as_ref() {
                result.add_string("nextCursor", cursor);
            }
        }

        let response = build_jsonrpc_response(&state.id, &result.build());
        write_sse_message(&state.out, &response)
    }
}

// ===== Content Tool Writer =====

impl content_tool_writer::Guest for ContentToolWriter {
    fn send_text(
        id: Id,
        out: OutputStream,
        text: String,
    ) -> Result<(), StreamError> {
        let mut content = JsonObjectBuilder::new();
        content.add_string("type", "text");
        content.add_string("text", &text);

        let mut result = JsonObjectBuilder::new();
        result.add_field("content", &format!("[{}]", content.build()));

        let response = build_jsonrpc_response(&id, &result.build());
        write_sse_message(&out, &response)
    }

    fn send_error(
        id: Id,
        out: OutputStream,
        reason: String,
    ) -> Result<(), StreamError> {
        let mut content = JsonObjectBuilder::new();
        content.add_string("type", "text");
        content.add_string("text", &reason);

        let mut result = JsonObjectBuilder::new();
        result.add_field("content", &format!("[{}]", content.build()));
        result.add_bool("isError", true);

        let response = build_jsonrpc_response(&id, &result.build());
        write_sse_message(&out, &response)
    }

    fn send_contents(
        id: Id,
        out: OutputStream,
        content: Vec<ContentBlock>,
    ) -> Result<(), StreamError> {
        let content_json = build_content_blocks_array(&content);

        let mut result = JsonObjectBuilder::new();
        result.add_field("content", &content_json);

        let response = build_jsonrpc_response(&id, &result.build());
        write_sse_message(&out, &response)
    }

    type Writer = ContentToolWriterResource;

    fn open(
        id: Id,
        out: OutputStream,
        initial: ContentBlock,
    ) -> Result<content_tool_writer::Writer, StreamError> {
        Ok(content_tool_writer::Writer::new(ContentToolWriterResource {
            state: RefCell::new(ContentWriterState {
                id,
                out,
                blocks: vec![initial],
                current_text: None,
            }),
        }))
    }
}

pub struct ContentToolWriterResource {
    state: RefCell<ContentWriterState>,
}

struct ContentWriterState {
    id: Id,
    out: OutputStream,
    blocks: Vec<ContentBlock>,
    current_text: Option<String>,
}

impl content_tool_writer::GuestWriter for ContentToolWriterResource {
    fn write(&self, contents: Vec<u8>) -> Result<(), StreamError> {
        let mut state = self.state.borrow_mut();

        // Convert bytes to string and append to current text
        // If not valid UTF-8, we'll need to handle it as a binary block later
        match String::from_utf8(contents) {
            Ok(text) => {
                match &mut state.current_text {
                    Some(current) => current.push_str(&text),
                    None => state.current_text = Some(text),
                }
                Ok(())
            }
            Err(_) => {
                // Invalid UTF-8 indicates binary data
                // This should be handled differently in production
                Err(StreamError::Closed)
            }
        }
    }

    fn next(&self, content: ContentBlock) -> Result<(), StreamError> {
        let mut state = self.state.borrow_mut();

        // If we have accumulated text, create a text block
        if let Some(text) = state.current_text.take() {
            state.blocks.push(ContentBlock::Text(
                crate::bindings::wasmcp::mcp::protocol::TextContent {
                    text,
                    options: None,
                }
            ));
        }

        state.blocks.push(content);
        Ok(())
    }

    fn close(&self, options: Option<content_tool_writer::Options>) -> Result<(), StreamError> {
        let mut state = self.state.borrow_mut();

        // If we have accumulated text, create a final text block
        if let Some(text) = state.current_text.take() {
            state.blocks.push(ContentBlock::Text(
                crate::bindings::wasmcp::mcp::protocol::TextContent {
                    text,
                    options: None,
                }
            ));
        }

        let content_json = build_content_blocks_array(&state.blocks);

        let mut result = JsonObjectBuilder::new();
        result.add_field("content", &content_json);

        if let Some(opts) = options {
            if opts.is_error {
                result.add_bool("isError", true);
            }
            if let Some(meta) = opts.meta {
                if !meta.is_empty() {
                    result.add_field("_meta", &build_meta_object(&meta));
                }
            }
        }

        let response = build_jsonrpc_response(&state.id, &result.build());
        write_sse_message(&state.out, &response)
    }
}

// ===== Structured Tool Writer =====

impl structured_tool_writer::Guest for StructuredToolWriter {
    fn send(
        id: Id,
        out: OutputStream,
        structured: structured_tool_writer::StructuredResult,
    ) -> Result<(), StreamError> {
        let mut result = JsonObjectBuilder::new();

        // The structured field is already a JSON string
        result.add_field("structured", &structured.structured);

        if let Some(opts) = structured.options {
            if opts.is_error {
                result.add_bool("isError", true);
            }
            if let Some(meta) = opts.meta {
                if !meta.is_empty() {
                    result.add_field("_meta", &build_meta_object(&meta));
                }
            }
        }

        let response = build_jsonrpc_response(&id, &result.build());
        write_sse_message(&out, &response)
    }
}

// ===== Helper Functions =====

/// Build a JSON array of tools.
fn build_tools_array(tools: &[list_tools_writer::Tool]) -> String {
    if tools.is_empty() {
        return "[]".to_string();
    }

    let tool_objects: Vec<String> = tools.iter().map(|tool| {
        let mut obj = JsonObjectBuilder::new();
        obj.add_string("name", &tool.name);
        obj.add_field("inputSchema", &tool.input_schema);

        if let Some(opts) = &tool.options {
            obj.add_optional_string("description", opts.description.as_deref());
            obj.add_optional_string("title", opts.title.as_deref());

            if let Some(schema) = &opts.output_schema {
                obj.add_field("outputSchema", schema);
            }

            if let Some(meta) = &opts.meta {
                if !meta.is_empty() {
                    obj.add_field("_meta", &build_meta_object(meta));
                }
            }

            if let Some(annotations) = &opts.annotations {
                let ann_json = build_tool_annotations(annotations);
                if ann_json != "{}" {
                    obj.add_field("annotations", &ann_json);
                }
            }
        }

        obj.build()
    }).collect();

    format!("[{}]", tool_objects.join(","))
}

/// Build JSON for tool annotations.
fn build_tool_annotations(annotations: &list_tools_writer::ToolAnnotations) -> String {
    let mut obj = JsonObjectBuilder::new();

    if let Some(title) = &annotations.title {
        obj.add_string("title", title);
    }

    // Convert hints flags to array of strings
    if !annotations.hints.is_empty() {
        let mut hints = Vec::new();
        if annotations.hints.contains(list_tools_writer::ToolHints::DESTRUCTIVE) {
            hints.push(r#""destructive""#);
        }
        if annotations.hints.contains(list_tools_writer::ToolHints::IDEMPOTENT) {
            hints.push(r#""idempotent""#);
        }
        if annotations.hints.contains(list_tools_writer::ToolHints::OPEN_WORLD) {
            hints.push(r#""open-world""#);
        }
        if annotations.hints.contains(list_tools_writer::ToolHints::READ_ONLY) {
            hints.push(r#""read-only""#);
        }

        if !hints.is_empty() {
            obj.add_field("hints", &format!("[{}]", hints.join(",")));
        }
    }

    obj.build()
}

/// Build a JSON array of content blocks.
fn build_content_blocks_array(blocks: &[ContentBlock]) -> String {
    if blocks.is_empty() {
        return "[]".to_string();
    }

    let block_objects: Vec<String> = blocks.iter()
        .map(build_content_block_json)
        .collect();

    format!("[{}]", block_objects.join(","))
}