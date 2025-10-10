//! Tools response writers
//!
//! Implements serialization for tool-related MCP responses including:
//! - Tool listings with pagination support
//! - Tool call results (text, content blocks, structured)
//! - Streaming writers for incremental output

use crate::bindings::exports::wasmcp::mcp::tools_response::{self, ContentBlocksResultOptions};
use crate::bindings::wasmcp::mcp::output::{
    finish_message, start_message, write_message_contents, IoError,
};
use crate::bindings::wasmcp::mcp::protocol::{
    ContentBlock, Id, NextCursorOptions, StructuredToolResult, Tool,
};
use crate::utils::{build_content_block_json, escape_json_string, JsonObjectBuilder};
use std::cell::RefCell;

// === Simple Response Functions ===

/// Write a tools/list response with complete tool list.
pub fn write_tools(id: Id, tools: Vec<Tool>) -> Result<(), IoError> {
    let tools_json = build_tools_array(&tools);

    let mut result = JsonObjectBuilder::new();
    result.add_raw_json("tools", &tools_json);

    let response = build_json_rpc_response(&id, &result.build());

    start_message()?;
    write_message_contents(&response.into_bytes())?;
    finish_message()
}

/// Write a simple text tool call result.
pub fn write_text(id: Id, text: String) -> Result<(), IoError> {
    let mut result = JsonObjectBuilder::new();
    result.add_raw_json(
        "content",
        &format!(
            r#"[{{"type":"text","text":"{}"}}]"#,
            escape_json_string(&text)
        ),
    );

    let response = build_json_rpc_response(&id, &result.build());

    start_message()?;
    write_message_contents(&response.into_bytes())?;
    finish_message()
}

/// Write an error tool call result.
pub fn write_error(id: Id, reason: String) -> Result<(), IoError> {
    let mut result = JsonObjectBuilder::new();
    result.add_bool("isError", true);
    result.add_raw_json(
        "content",
        &format!(
            r#"[{{"type":"text","text":"{}"}}]"#,
            escape_json_string(&reason)
        ),
    );

    let response = build_json_rpc_response(&id, &result.build());

    start_message()?;
    write_message_contents(&response.into_bytes())?;
    finish_message()
}

/// Write tool call result with content blocks.
pub fn write_content_blocks(id: Id, content: Vec<ContentBlock>) -> Result<(), IoError> {
    let content_json = build_content_blocks_array(&content);

    let mut result = JsonObjectBuilder::new();
    result.add_raw_json("content", &content_json);

    let response = build_json_rpc_response(&id, &result.build());

    start_message()?;
    write_message_contents(&response.into_bytes())?;
    finish_message()
}

/// Write a structured tool call result.
pub fn write_structured(id: Id, structured: StructuredToolResult) -> Result<(), IoError> {
    let mut result = JsonObjectBuilder::new();

    result.add_validated_json("content", &structured.structured_content);

    if structured.is_error {
        result.add_bool("isError", true);
    }

    if let Some(meta) = &structured.meta {
        if !meta.is_empty() {
            result.add_raw_json("_meta", &build_meta_json(meta));
        }
    }

    let response = build_json_rpc_response(&id, &result.build());

    start_message()?;
    write_message_contents(&response.into_bytes())?;
    finish_message()
}

// === Streaming Tools Writer Resource ===

pub struct ToolsWriter {
    state: RefCell<ToolsWriterState>,
}

struct ToolsWriterState {
    first_item: bool,
}

impl crate::bindings::exports::wasmcp::mcp::tools_response::GuestToolsWriter for ToolsWriter {
    fn start(id: Id) -> Result<tools_response::ToolsWriter, IoError> {
        let id_str = format_id(&id);
        let header = format!(r#"{{"jsonrpc":"2.0","id":{id_str},"result":{{"tools":["#);

        start_message()?;
        write_message_contents(&header.into_bytes())?;

        // Try constructing wrapper with new()
        Ok(tools_response::ToolsWriter::new(ToolsWriter {
            state: RefCell::new(ToolsWriterState { first_item: true }),
        }))
    }

    fn write(&self, tool: Tool) -> Result<(), IoError> {
        let mut state = self.state.borrow_mut();

        let mut output_str = String::new();
        if !state.first_item {
            output_str.push(',');
        } else {
            state.first_item = false;
        }

        let tool_json = build_single_tool(&tool);
        output_str.push_str(&tool_json);

        write_message_contents(&output_str.into_bytes())
    }

    fn finish(
        _this: tools_response::ToolsWriter,
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

// === Streaming Content Blocks Writer Resource ===

pub struct ContentBlocksWriter;

impl crate::bindings::exports::wasmcp::mcp::tools_response::GuestContentBlocksWriter
    for ContentBlocksWriter
{
    fn start(
        id: Id,
        initial: ContentBlock,
    ) -> Result<tools_response::ContentBlocksWriter, IoError> {
        let id_str = format_id(&id);
        let header = format!(r#"{{"jsonrpc":"2.0","id":{id_str},"result":{{"content":["#);

        let initial_json = build_content_block_json(&initial);

        start_message()?;
        write_message_contents(&header.into_bytes())?;
        write_message_contents(&initial_json.into_bytes())?;

        Ok(tools_response::ContentBlocksWriter::new(
            ContentBlocksWriter,
        ))
    }

    fn write(&self, contents: Vec<u8>) -> Result<(), IoError> {
        // Raw binary data write (for streaming large content)
        write_message_contents(&contents)
    }

    fn next(&self, content: ContentBlock) -> Result<(), IoError> {
        let content_json = build_content_block_json(&content);
        let chunk = format!(",{}", content_json);
        write_message_contents(&chunk.into_bytes())
    }

    fn finish(
        _this: tools_response::ContentBlocksWriter,
        options: Option<ContentBlocksResultOptions>,
    ) -> Result<(), IoError> {
        let mut closing = String::from("]");

        if let Some(opts) = options {
            closing.push_str(&format!(r#","isError":{}"#, opts.is_error));

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

fn build_tools_array(tools: &[Tool]) -> String {
    if tools.is_empty() {
        return "[]".to_string();
    }

    let tool_jsons: Vec<String> = tools.iter().map(build_single_tool).collect();
    format!("[{}]", tool_jsons.join(","))
}

fn build_single_tool(tool: &Tool) -> String {
    let mut obj = JsonObjectBuilder::new();
    obj.add_string("name", &tool.name);
    obj.add_validated_json("inputSchema", &tool.input_schema);

    if let Some(opts) = &tool.options {
        if let Some(description) = &opts.description {
            obj.add_string("description", description);
        }
        if let Some(title) = &opts.title {
            obj.add_string("title", title);
        }
        if let Some(output_schema) = &opts.output_schema {
            obj.add_validated_json("outputSchema", output_schema);
        }
        if let Some(annotations) = &opts.annotations {
            obj.add_raw_json("annotations", &build_tool_annotations_json(annotations));
        }
        if let Some(meta) = &opts.meta {
            if !meta.is_empty() {
                obj.add_raw_json("_meta", &build_meta_json(meta));
            }
        }
    }

    obj.build()
}

fn build_tool_annotations_json(
    ann: &crate::bindings::wasmcp::mcp::protocol::ToolAnnotations,
) -> String {
    let mut obj = JsonObjectBuilder::new();

    if let Some(title) = &ann.title {
        obj.add_string("title", title);
    }
    if let Some(read_only) = ann.read_only_hint {
        obj.add_bool("readOnlyHint", read_only);
    }
    if let Some(destructive) = ann.destructive_hint {
        obj.add_bool("destructiveHint", destructive);
    }
    if let Some(idempotent) = ann.idempotent_hint {
        obj.add_bool("idempotentHint", idempotent);
    }
    if let Some(open_world) = ann.open_world_hint {
        obj.add_bool("openWorldHint", open_world);
    }

    obj.build()
}

fn build_content_blocks_array(blocks: &[ContentBlock]) -> String {
    if blocks.is_empty() {
        return "[]".to_string();
    }

    let block_jsons: Vec<String> = blocks.iter().map(build_content_block_json).collect();
    format!("[{}]", block_jsons.join(","))
}

fn build_meta_json(meta: &[(String, String)]) -> String {
    let mut obj = JsonObjectBuilder::new();
    for (key, value) in meta {
        obj.add_string(key, value);
    }
    obj.build()
}

fn build_json_rpc_response(id: &Id, result: &str) -> String {
    let id_str = format_id(id);
    format!(r#"{{"jsonrpc":"2.0","id":{},"result":{}}}"#, id_str, result)
}

fn format_id(id: &Id) -> String {
    match id {
        Id::Number(n) => n.to_string(),
        Id::String(s) => format!(r#""{}""#, escape_json_string(s)),
    }
}
