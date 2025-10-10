//! Shared utilities for MCP response serialization.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::Value;

/// Escape a string for JSON.
///
/// Handles all required JSON escapes per RFC 8259.
pub fn escape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());

    for c in s.chars() {
        match c {
            '"' => result.push_str(r#"\""#),
            '\\' => result.push_str(r#"\\"#),
            '\n' => result.push_str(r#"\n"#),
            '\r' => result.push_str(r#"\r"#),
            '\t' => result.push_str(r#"\t"#),
            '\u{0008}' => result.push_str(r#"\b"#),
            '\u{000C}' => result.push_str(r#"\f"#),
            c if c.is_control() => {
                result.push_str(&format!(r#"\u{:04x}"#, c as u32));
            }
            c => result.push(c),
        }
    }

    result
}

/// Validates and compacts handler-provided JSON.
///
/// Handlers provide pre-serialized JSON for fields like `input_schema` and
/// `structured_content`. This function ensures they contain valid, compact JSON
/// without embedded newlines that would corrupt stdio transport framing.
///
/// # Panics
/// Panics if the JSON is invalid. This is intentional: invalid JSON from a handler
/// is a critical bug that must be fixed at the source, not silently papered over.
pub fn compact_json(json_str: &str) -> String {
    let value: Value = serde_json::from_str(json_str).unwrap_or_else(|e| {
        panic!(
            "Handler provided invalid JSON: {}. Original: {:?}",
            e, json_str
        )
    });

    serde_json::to_string(&value).expect("serde_json serialization is infallible for Value")
}

/// Base64 encode bytes using the standard encoding.
///
/// Uses the standard base64 alphabet with padding, as per RFC 4648.
#[inline]
pub fn base64_encode(data: &[u8]) -> String {
    BASE64.encode(data)
}

/// Build JSON for a single content block.
///
/// Handles all content block types: text, image, audio, resource-link, embedded-resource.
pub fn build_content_block_json(
    block: &crate::bindings::wasmcp::mcp::protocol::ContentBlock,
) -> String {
    use crate::bindings::wasmcp::mcp::protocol::{ContentBlock, EmbeddedResource};

    let mut obj = JsonObjectBuilder::new();

    match block {
        ContentBlock::Text(text) => {
            obj.add_string("type", "text");
            obj.add_string("text", &text.text);
        }
        ContentBlock::Image(image) => {
            obj.add_string("type", "image");
            obj.add_string("data", &base64_encode(&image.data));
            obj.add_string("mimeType", &image.mime_type);
        }
        ContentBlock::Audio(audio) => {
            obj.add_string("type", "audio");
            obj.add_string("data", &base64_encode(&audio.data));
            obj.add_string("mimeType", &audio.mime_type);
        }
        ContentBlock::ResourceLink(link) => {
            obj.add_string("type", "resource");
            obj.add_string("uri", &link.uri);
            obj.add_string("name", &link.name);
        }
        ContentBlock::EmbeddedResource(embedded) => {
            obj.add_string("type", "resource");

            match &embedded.resource {
                EmbeddedResource::Text(text_res) => {
                    obj.add_string("uri", &text_res.uri);
                    obj.add_string("text", &text_res.text);
                    if let Some(opts) = &text_res.options {
                        if let Some(mime) = &opts.mime_type {
                            obj.add_string("mimeType", mime);
                        }
                    }
                }
                EmbeddedResource::Blob(blob_res) => {
                    obj.add_string("uri", &blob_res.uri);
                    obj.add_string("blob", &base64_encode(&blob_res.blob));
                    if let Some(opts) = &blob_res.options {
                        if let Some(mime) = &opts.mime_type {
                            obj.add_string("mimeType", mime);
                        }
                    }
                }
            }
        }
    }

    obj.build()
}

/// Builder for constructing JSON objects with type-safe field addition.
pub struct JsonObjectBuilder {
    fields: Vec<String>,
}

impl JsonObjectBuilder {
    pub fn new() -> Self {
        Self {
            fields: Vec::with_capacity(8),
        }
    }

    /// Add a raw JSON value (object, array, etc).
    ///
    /// For handler-provided JSON, use `add_validated_json` instead.
    pub fn add_raw_json(&mut self, name: &str, json_value: &str) {
        self.fields.push(format!("\"{}\":{}", name, json_value));
    }

    /// Add handler-provided JSON with validation and compaction.
    ///
    /// This validates the JSON structure and removes any embedded newlines
    /// that would corrupt stdio transport framing.
    pub fn add_validated_json(&mut self, name: &str, json_value: &str) {
        let compacted = compact_json(json_value);
        self.fields.push(format!("\"{}\":{}", name, compacted));
    }

    /// Add a string field with proper JSON escaping.
    pub fn add_string(&mut self, name: &str, value: &str) {
        self.fields
            .push(format!("\"{}\":\"{}\"", name, escape_json_string(value)));
    }

    /// Add a boolean field.
    pub fn add_bool(&mut self, name: &str, value: bool) {
        self.fields.push(format!("\"{}\":{}", name, value));
    }

    /// Add a numeric field.
    pub fn add_number(&mut self, name: &str, value: impl std::fmt::Display) {
        self.fields.push(format!("\"{}\":{}", name, value));
    }

    /// Build the final JSON object string.
    pub fn build(self) -> String {
        format!("{{{}}}", self.fields.join(","))
    }
}

impl Default for JsonObjectBuilder {
    fn default() -> Self {
        Self::new()
    }
}
