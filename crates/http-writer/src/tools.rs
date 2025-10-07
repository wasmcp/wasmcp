//! Tools writer implementations for HTTP/SSE transport.
//!
//! Handles serialization of tool-related responses including:
//! - Tool listings with schemas and metadata
//! - Tool execution results (both structured and unstructured)
//! - TRUE STREAMING output with proper WASI I/O compliance

use crate::bindings::exports::wasmcp::mcp::{
    list_tools_writer, content_tool_writer, structured_tool_writer,
};
use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use crate::bindings::wasmcp::mcp::protocol::{ContentBlock, Id};
use crate::utils::{
    build_content_block_json, build_jsonrpc_response, format_id,
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
        // One-shot: Build complete response and send
        let tools_json = build_tools_array(&tools);

        let mut result = JsonObjectBuilder::new();
        result.add_field("tools", &tools_json);

        let response = build_jsonrpc_response(&id, &result.build());
        write_sse_message(&out, &response)?;

        // Flush to ensure delivery
        out.flush()?;
        Ok(())
    }

    type Writer = ListToolsWriterResource;

    fn open(
        id: Id,
        out: OutputStream,
    ) -> Result<list_tools_writer::Writer, StreamError> {
        // Start the JSON-RPC response and tools array
        let id_str = format_id(&id);
        let header = format!(
            r#"{{"jsonrpc":"2.0","id":{id_str},"result":{{"tools":["#
        );

        // Write the opening of the response
        write_sse_message(&out, &header)?;

        Ok(list_tools_writer::Writer::new(ListToolsWriterResource {
            state: RefCell::new(StreamingWriterState {
                out,
                first_item: true,
                closed: false,
            }),
        }))
    }
}

pub struct ListToolsWriterResource {
    state: RefCell<StreamingWriterState>,
}

struct StreamingWriterState {
    out: OutputStream,
    first_item: bool,
    closed: bool,
}

impl list_tools_writer::GuestWriter for ListToolsWriterResource {
    fn write(&self, tool: list_tools_writer::Tool) -> Result<(), StreamError> {
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

        // Build and append this single tool
        let tool_json = build_single_tool(&tool);
        output.push_str(&tool_json);

        // Write immediately - true streaming!
        write_sse_message(&state.out, &output)?;

        Ok(())
    }

    fn close(&self, options: Option<list_tools_writer::Options>) -> Result<(), StreamError> {
        let mut state = self.state.borrow_mut();

        if state.closed {
            return Err(StreamError::Closed);
        }

        // Close the tools array and add optional fields
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
                closing.push_str(&escape_json(cursor));
                closing.push('"');
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
        write_sse_message(&out, &response)?;
        out.flush()?;
        Ok(())
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
        write_sse_message(&out, &response)?;
        out.flush()?;
        Ok(())
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
        write_sse_message(&out, &response)?;
        out.flush()?;
        Ok(())
    }

    type Writer = ContentToolWriterResource;

    fn open(
        id: Id,
        out: OutputStream,
        initial: ContentBlock,
    ) -> Result<content_tool_writer::Writer, StreamError> {
        // Start the response with the first content block
        let id_str = format_id(&id);
        let header = format!(
            r#"{{"jsonrpc":"2.0","id":{id_str},"result":{{"content":["#
        );
        write_sse_message(&out, &header)?;

        // Write the initial block
        let initial_json = build_content_block_json(&initial);
        write_sse_message(&out, &initial_json)?;

        Ok(content_tool_writer::Writer::new(ContentToolWriterResource {
            state: RefCell::new(ContentStreamingState {
                out,
                closed: false,
            }),
        }))
    }
}

pub struct ContentToolWriterResource {
    state: RefCell<ContentStreamingState>,
}

struct ContentStreamingState {
    out: OutputStream,
    closed: bool,
}

impl content_tool_writer::GuestWriter for ContentToolWriterResource {
    fn write(&self, contents: Vec<u8>) -> Result<(), StreamError> {
        let state = self.state.borrow_mut();

        if state.closed {
            return Err(StreamError::Closed);
        }

        // For raw bytes, we stream them as base64-encoded chunks
        // This enables true streaming of large binary content
        if !contents.is_empty() {
            let encoded = crate::utils::base64_encode(&contents);
            write_sse_message(&state.out, &encoded)?;
        }

        Ok(())
    }

    fn next(&self, content: ContentBlock) -> Result<(), StreamError> {
        let state = self.state.borrow_mut();

        if state.closed {
            return Err(StreamError::Closed);
        }

        // Add a new content block to the stream
        let block_json = build_content_block_json(&content);
        let output = format!(",{}", block_json);
        write_sse_message(&state.out, &output)?;

        Ok(())
    }

    fn close(&self, options: Option<content_tool_writer::Options>) -> Result<(), StreamError> {
        let mut state = self.state.borrow_mut();

        if state.closed {
            return Err(StreamError::Closed);
        }

        // Close the content array and add options
        let mut closing = String::from("]");

        if let Some(opts) = options {
            if opts.is_error {
                closing.push_str(r#","isError":true"#);
            }
            if let Some(meta) = opts.meta {
                if !meta.is_empty() {
                    closing.push_str(r#","_meta":"#);
                    closing.push_str(&build_meta_object(&meta));
                }
            }
        }

        closing.push_str("}}");
        write_sse_message(&state.out, &closing)?;

        state.out.flush()?;
        state.closed = true;

        Ok(())
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
        write_sse_message(&out, &response)?;
        out.flush()?;
        Ok(())
    }
}

// ===== Helper Functions =====

/// Build a JSON array of tools (for one-shot send).
fn build_tools_array(tools: &[list_tools_writer::Tool]) -> String {
    if tools.is_empty() {
        return "[]".to_string();
    }

    let tool_objects: Vec<String> = tools.iter()
        .map(build_single_tool)
        .collect();

    format!("[{}]", tool_objects.join(","))
}

/// Build JSON for a single tool.
fn build_single_tool(tool: &list_tools_writer::Tool) -> String {
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

/// Simple JSON string escaping
fn escape_json(s: &str) -> String {
    crate::utils::escape_json_string(s)
}