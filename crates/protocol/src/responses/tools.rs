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
    ContentBlock, Id, NextCursorOptions, StructuredToolResult, TextContent, Tool,
};
use crate::utils::{build_content_block_json, compact_json, escape_json_string, JsonObjectBuilder};
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
///
/// Per MCP spec: "For backwards compatibility, a tool that returns structured content
/// SHOULD also return the serialized JSON in a TextContent block."
/// https://modelcontextprotocol.io/specification/draft/server/tools#structured-content
///
/// Handlers can optionally provide supplementary content blocks (images, resources)
/// alongside the structured data via the supplementary-content field.
pub fn write_structured(id: Id, structured: StructuredToolResult) -> Result<(), IoError> {
    let mut result = JsonObjectBuilder::new();

    // Build content array: backwards-compat text block + optional supplementary content
    let mut content_blocks = Vec::new();

    // 1. Backwards compatibility: add serialized JSON as text block
    content_blocks.push(ContentBlock::Text(TextContent {
        text: structured.structured_content.clone(),
        options: None,
    }));

    // 2. Add any supplementary content blocks the handler provided
    if let Some(supplementary) = structured.supplementary_content {
        content_blocks.extend(supplementary);
    }

    let content_json = build_content_blocks_array(&content_blocks);
    result.add_raw_json("content", &content_json);

    // Add the actual structured content in its own field
    result.add_validated_json("structuredContent", &structured.structured_content);

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

pub struct ContentBlocksWriter {
    state: RefCell<ContentBlocksWriterState>,
}

struct ContentBlocksWriterState {
    first_block: bool,
}

impl crate::bindings::exports::wasmcp::mcp::tools_response::GuestContentBlocksWriter
    for ContentBlocksWriter
{
    fn start(id: Id) -> Result<tools_response::ContentBlocksWriter, IoError> {
        let id_str = format_id(&id);
        let header = format!(r#"{{"jsonrpc":"2.0","id":{id_str},"result":{{"content":["#);

        start_message()?;
        write_message_contents(&header.into_bytes())?;

        Ok(tools_response::ContentBlocksWriter::new(
            ContentBlocksWriter {
                state: RefCell::new(ContentBlocksWriterState { first_block: true }),
            },
        ))
    }

    fn add_text(&self, text: String) -> Result<(), IoError> {
        self.write_comma_if_needed()?;

        let mut obj = crate::utils::JsonObjectBuilder::new();
        obj.add_string("type", "text");
        obj.add_string("text", &text);

        write_message_contents(&obj.build().into_bytes())
    }

    fn add_image(&self, data: Vec<u8>, mime_type: String) -> Result<(), IoError> {
        self.write_comma_if_needed()?;

        let mut obj = crate::utils::JsonObjectBuilder::new();
        obj.add_string("type", "image");
        obj.add_string("data", &crate::utils::base64_encode(&data));
        obj.add_string("mimeType", &mime_type);

        write_message_contents(&obj.build().into_bytes())
    }

    fn add_image_stream(
        &self,
        mime_type: String,
        source: &crate::bindings::wasi::io::streams::InputStream,
    ) -> Result<u64, IoError> {
        self.write_comma_if_needed()?;

        // Write block opening
        write_message_contents(br#"{"type":"image","data":""#)?;

        // Stream base64-encode
        let bytes_read = self.stream_base64_encode(source)?;

        // Close block
        write_message_contents(
            format!(r#"","mimeType":"{}"}}"#, crate::utils::escape_json_string(&mime_type))
                .as_bytes(),
        )?;

        Ok(bytes_read)
    }

    fn add_audio(&self, data: Vec<u8>, mime_type: String) -> Result<(), IoError> {
        self.write_comma_if_needed()?;

        let mut obj = crate::utils::JsonObjectBuilder::new();
        obj.add_string("type", "audio");
        obj.add_string("data", &crate::utils::base64_encode(&data));
        obj.add_string("mimeType", &mime_type);

        write_message_contents(&obj.build().into_bytes())
    }

    fn add_audio_stream(
        &self,
        mime_type: String,
        source: &crate::bindings::wasi::io::streams::InputStream,
    ) -> Result<u64, IoError> {
        self.write_comma_if_needed()?;

        // Write block opening
        write_message_contents(br#"{"type":"audio","data":""#)?;

        // Stream base64-encode
        let bytes_read = self.stream_base64_encode(source)?;

        // Close block
        write_message_contents(
            format!(r#"","mimeType":"{}"}}"#, crate::utils::escape_json_string(&mime_type))
                .as_bytes(),
        )?;

        Ok(bytes_read)
    }

    fn add_resource_link(&self, uri: String, name: String) -> Result<(), IoError> {
        self.write_comma_if_needed()?;

        let mut obj = crate::utils::JsonObjectBuilder::new();
        obj.add_string("type", "resource");
        obj.add_string("uri", &uri);
        obj.add_string("name", &name);

        write_message_contents(&obj.build().into_bytes())
    }

    fn add_embedded_resource_text(
        &self,
        uri: String,
        text: String,
        mime_type: Option<String>,
    ) -> Result<(), IoError> {
        self.write_comma_if_needed()?;

        let mut obj = crate::utils::JsonObjectBuilder::new();
        obj.add_string("type", "resource");
        obj.add_string("uri", &uri);
        obj.add_string("text", &text);
        if let Some(mt) = mime_type {
            obj.add_string("mimeType", &mt);
        }

        write_message_contents(&obj.build().into_bytes())
    }

    fn add_embedded_resource_blob(
        &self,
        uri: String,
        blob: Vec<u8>,
        mime_type: Option<String>,
    ) -> Result<(), IoError> {
        self.write_comma_if_needed()?;

        let mut obj = crate::utils::JsonObjectBuilder::new();
        obj.add_string("type", "resource");
        obj.add_string("uri", &uri);
        obj.add_string("blob", &crate::utils::base64_encode(&blob));
        if let Some(mt) = mime_type {
            obj.add_string("mimeType", &mt);
        }

        write_message_contents(&obj.build().into_bytes())
    }

    fn add_embedded_resource_blob_stream(
        &self,
        uri: String,
        mime_type: Option<String>,
        source: &crate::bindings::wasi::io::streams::InputStream,
    ) -> Result<u64, IoError> {
        self.write_comma_if_needed()?;

        // Write block opening
        write_message_contents(
            format!(r#"{{"type":"resource","uri":"{}","blob":""#, crate::utils::escape_json_string(&uri))
                .as_bytes(),
        )?;

        // Stream base64-encode
        let bytes_read = self.stream_base64_encode(source)?;

        // Close block
        if let Some(mt) = mime_type {
            write_message_contents(
                format!(r#"","mimeType":"{}"}}"#, crate::utils::escape_json_string(&mt))
                    .as_bytes(),
            )?;
        } else {
            write_message_contents(br#""}"#)?;
        }

        Ok(bytes_read)
    }

    fn finish(
        _this: tools_response::ContentBlocksWriter,
        options: Option<ContentBlocksResultOptions>,
    ) -> Result<(), IoError> {
        let mut closing = String::from("]");

        if let Some(opts) = options {
            closing.push_str(&format!(r#","isError":{}"#, opts.is_error));

            // Add structuredContent if provided (allows content-blocks responses to include structured data)
            if let Some(structured) = opts.structured_content {
                closing.push_str(r#","structuredContent":"#);
                closing.push_str(&compact_json(&structured));
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

impl ContentBlocksWriter {
    /// Write comma if this is not the first block
    fn write_comma_if_needed(&self) -> Result<(), IoError> {
        let mut state = self.state.borrow_mut();
        if !state.first_block {
            write_message_contents(b",")?;
        }
        state.first_block = false;
        Ok(())
    }

    /// Stream base64-encode data from input-stream with bounded memory.
    ///
    /// Reads 4KB chunks, encodes incrementally, and writes to output.
    /// Returns total bytes read from stream.
    fn stream_base64_encode(
        &self,
        source: &crate::bindings::wasi::io::streams::InputStream,
    ) -> Result<u64, IoError> {
        use crate::bindings::wasi::io::streams::StreamError;

        let mut encoder = crate::utils::Base64StreamEncoder::new();
        let mut total_bytes: u64 = 0;

        loop {
            // Read chunk with blocking (up to 4KB)
            let chunk = source
                .blocking_read(4096)
                .map_err(|e| match e {
                    StreamError::LastOperationFailed(err) => IoError::Stream(StreamError::LastOperationFailed(err)),
                    StreamError::Closed => IoError::Stream(StreamError::Closed),
                })?;

            if chunk.is_empty() {
                break;
            }

            total_bytes += chunk.len() as u64;

            // Encode and write chunk
            let encoded = encoder.encode_chunk(&chunk);
            if !encoded.is_empty() {
                write_message_contents(encoded.as_bytes())?;
            }
        }

        // Finalize encoding (write any remaining buffered bytes with padding)
        let final_encoded = encoder.finalize();
        if !final_encoded.is_empty() {
            write_message_contents(final_encoded.as_bytes())?;
        }

        Ok(total_bytes)
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
