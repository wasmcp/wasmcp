//! Shared utilities for HTTP writer implementations.
//!
//! This module contains common functions used across all writer implementations
//! for JSON serialization, SSE formatting, and stream management.

use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use crate::bindings::wasmcp::mcp::protocol::{Id, Meta, Annotations, Role};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

/// Write SSE-formatted data to a stream.
///
/// Formats the data according to Server-Sent Events specification:
/// - Each line prefixed with "data: "
/// - Message ends with double newline
/// - Handles multi-line JSON properly
///
/// Also implements proper check-write semantics for WASI I/O streams.
pub fn write_sse_message(stream: &OutputStream, data: &str) -> Result<(), StreamError> {
    // For SSE, we need to prefix each line with "data: "
    // and end with double newline
    let mut sse_formatted = String::with_capacity(data.len() + (data.lines().count() * 6) + 2);

    // Split the JSON by newlines in case it's pretty-printed
    for line in data.lines() {
        sse_formatted.push_str("data: ");
        sse_formatted.push_str(line);
        sse_formatted.push('\n');
    }

    // Add the final newline to create the double newline that ends an SSE message
    sse_formatted.push('\n');

    // Check available space before writing
    let bytes = sse_formatted.as_bytes();
    write_with_backpressure(stream, bytes)?;

    stream.flush()?;
    Ok(())
}

/// Write bytes to stream with proper backpressure handling.
///
/// Respects the check-write contract by:
/// 1. Checking available space before writing
/// 2. Writing in chunks if necessary
/// 3. Handling zero-availability gracefully
pub fn write_with_backpressure(stream: &OutputStream, bytes: &[u8]) -> Result<(), StreamError> {
    let mut offset = 0;

    while offset < bytes.len() {
        // Check how much we can write
        let available = stream.check_write()?;

        if available == 0 {
            // Stream is not ready, this shouldn't happen in practice
            // as WASI runtime should handle blocking, but we handle it gracefully
            continue;
        }

        // Write up to the available amount
        let chunk_size = std::cmp::min(available as usize, bytes.len() - offset);
        stream.write(&bytes[offset..offset + chunk_size])?;
        offset += chunk_size;
    }

    Ok(())
}

/// Build a JSON-RPC 2.0 response wrapper.
pub fn build_jsonrpc_response(id: &Id, result: &str) -> String {
    let id_str = format_id(id);
    format!(
        r#"{{"jsonrpc":"2.0","id":{id_str},"result":{result}}}"#
    )
}

/// Build a JSON-RPC 2.0 error response.
pub fn build_jsonrpc_error(id: &Id, code: i32, message: &str, data: Option<&str>) -> String {
    let id_str = format_id(id);

    let mut error = format!(
        r#"{{"jsonrpc":"2.0","id":{},"error":{{"code":{},"message":"{}""#,
        id_str, code, escape_json_string(message)
    );

    if let Some(data) = data {
        error.push_str(r#","data":"#);
        error.push_str(data);
    }

    error.push_str("}}");
    error
}

/// Build a JSON-RPC 2.0 notification.
pub fn build_jsonrpc_notification(method: &str, params: Option<&str>) -> String {
    let mut notification = format!(
        r#"{{"jsonrpc":"2.0","method":"{}""#,
        escape_json_string(method)
    );

    if let Some(params) = params {
        notification.push_str(r#","params":"#);
        notification.push_str(params);
    }

    notification.push('}');
    notification
}

/// Format a JSON-RPC ID (string or number).
pub fn format_id(id: &Id) -> String {
    match id {
        Id::String(s) => format!(r#""{}""#, escape_json_string(s)),
        Id::Number(n) => n.to_string(),
    }
}

/// Escape a string for JSON.
///
/// Handles all required JSON escapes including:
/// - Quotes and backslashes
/// - Control characters
/// - Unicode escapes for non-printable characters
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
                // Unicode escape for other control characters
                result.push_str(&format!(r#"\u{:04x}"#, c as u32));
            }
            c => result.push(c),
        }
    }

    result
}

/// Convert a Meta hashmap to a JSON object string.
///
/// Returns "null" for None or empty, otherwise a JSON object.
pub fn meta_to_json(meta: &Meta) -> String {
    match meta {
        None => "null".to_string(),
        Some(entries) if entries.is_empty() => "null".to_string(),
        Some(entries) => {
            let json_entries: Vec<String> = entries
                .iter()
                .map(|(k, v)| format!(r#""{}":"{}""#, escape_json_string(k), escape_json_string(v)))
                .collect();

            format!("{{{}}}", json_entries.join(","))
        }
    }
}

/// Format optional meta field for inclusion in JSON.
///
/// Returns None if meta is None or empty, otherwise Some with the JSON string.
pub fn format_meta_field(meta: &Meta) -> Option<String> {
    match meta {
        None => None,
        Some(entries) if entries.is_empty() => None,
        Some(_) => Some(format!(r#""_meta":{}"#, meta_to_json(meta)))
    }
}

/// Build a meta object from key-value pairs for use in JSON objects.
pub fn build_meta_object(meta: &[(String, String)]) -> String {
    if meta.is_empty() {
        return "{}".to_string();
    }

    let mut obj = JsonObjectBuilder::new();
    for (key, value) in meta {
        obj.add_string(key, value);
    }
    obj.build()
}

/// Build JSON for annotations.
pub fn build_annotations_json(ann: &Annotations) -> String {
    let mut obj = JsonObjectBuilder::new();

    if let Some(audience) = &ann.audience {
        let audience_strs: Vec<String> = audience.iter().map(|role| {
            format!(r#""{}""#, match role {
                Role::User => "user",
                Role::Assistant => "assistant",
            })
        }).collect();
        obj.add_field("audience", &format!("[{}]", audience_strs.join(",")));
    }

    if let Some(last_mod) = &ann.last_modified {
        obj.add_string("lastModified", last_mod);
    }

    obj.add_number("priority", ann.priority);

    obj.build()
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
/// Handles all content block types: text, image, audio, resource links, and embedded resources.
pub fn build_content_block_json(block: &crate::bindings::wasmcp::mcp::protocol::ContentBlock) -> String {
    use crate::bindings::wasmcp::mcp::protocol::{ContentBlock, EmbeddedResource};

    let mut obj = JsonObjectBuilder::new();

    match block {
        ContentBlock::Text(text) => {
            obj.add_string("type", "text");
            obj.add_string("text", &text.text);
            if let Some(opts) = &text.options {
                add_content_options(&mut obj, opts);
            }
        }
        ContentBlock::Image(image) => {
            obj.add_string("type", "image");
            obj.add_string("data", &base64_encode(&image.data));
            obj.add_string("mimeType", &image.mime_type);
            if let Some(opts) = &image.options {
                add_content_options(&mut obj, opts);
            }
        }
        ContentBlock::Audio(audio) => {
            obj.add_string("type", "audio");
            obj.add_string("data", &base64_encode(&audio.data));
            obj.add_string("mimeType", &audio.mime_type);
            if let Some(opts) = &audio.options {
                add_content_options(&mut obj, opts);
            }
        }
        ContentBlock::ResourceLink(link) => {
            obj.add_string("type", "resource_link");
            obj.add_string("uri", &link.uri);
            obj.add_string("name", &link.name);

            if let Some(opts) = &link.options {
                obj.add_optional_string("title", opts.title.as_deref());
                obj.add_optional_string("description", opts.description.as_deref());
                obj.add_optional_number("size", opts.size);
                obj.add_optional_string("mimeType", opts.mime_type.as_deref());

                if let Some(ann) = &opts.annotations {
                    obj.add_field("annotations", &build_annotations_json(ann));
                }
                if let Some(meta) = &opts.meta {
                    if !meta.is_empty() {
                        obj.add_field("_meta", &build_meta_object(meta));
                    }
                }
            }
        }
        ContentBlock::EmbeddedResource(embedded) => {
            obj.add_string("type", "resource");
            match &embedded.resource {
                EmbeddedResource::Text(text) => {
                    obj.add_string("uri", &text.uri);
                    obj.add_string("text", &text.text);
                    if let Some(opts) = &text.options {
                        obj.add_optional_string("mimeType", opts.mime_type.as_deref());
                        if let Some(meta) = &opts.meta {
                            if !meta.is_empty() {
                                obj.add_field("_meta", &build_meta_object(meta));
                            }
                        }
                    }
                }
                EmbeddedResource::Blob(blob) => {
                    obj.add_string("uri", &blob.uri);
                    obj.add_string("blob", &base64_encode(&blob.blob));
                    if let Some(opts) = &blob.options {
                        obj.add_optional_string("mimeType", opts.mime_type.as_deref());
                        if let Some(meta) = &opts.meta {
                            if !meta.is_empty() {
                                obj.add_field("_meta", &build_meta_object(meta));
                            }
                        }
                    }
                }
            }
            if let Some(opts) = &embedded.options {
                add_content_options(&mut obj, opts);
            }
        }
    }

    obj.build()
}

/// Add content options to a JSON object builder.
fn add_content_options(
    obj: &mut JsonObjectBuilder,
    opts: &crate::bindings::wasmcp::mcp::protocol::ContentOptions
) {
    if let Some(ann) = &opts.annotations {
        obj.add_field("annotations", &build_annotations_json(ann));
    }
    if let Some(meta) = &opts.meta {
        if !meta.is_empty() {
            obj.add_field("_meta", &build_meta_object(meta));
        }
    }
}

/// Helper to build JSON objects with optional fields.
#[allow(dead_code)]
pub struct JsonObjectBuilder {
    fields: Vec<String>,
}

#[allow(dead_code)]
impl JsonObjectBuilder {
    pub fn new() -> Self {
        JsonObjectBuilder {
            fields: Vec::with_capacity(8), // Pre-allocate for typical use
        }
    }

    /// Add a required field.
    pub fn add_field(&mut self, name: &str, value: &str) {
        self.fields.push(format!(r#""{name}"":{value}"#));
    }

    /// Add an optional field (only if Some).
    pub fn add_optional_field(&mut self, name: &str, value: Option<String>) {
        if let Some(v) = value {
            self.fields.push(format!(r#""{name}"":{v}"#));
        }
    }

    /// Add a string field with proper escaping.
    pub fn add_string(&mut self, name: &str, value: &str) {
        self.fields.push(format!(r#""{}"":"{}""#, name, escape_json_string(value)));
    }

    /// Add an optional string field.
    pub fn add_optional_string(&mut self, name: &str, value: Option<&str>) {
        if let Some(v) = value {
            self.add_string(name, v);
        }
    }

    /// Add a boolean field.
    pub fn add_bool(&mut self, name: &str, value: bool) {
        self.fields.push(format!(r#""{name}"":{value}"#));
    }

    /// Add an optional boolean field.
    pub fn add_optional_bool(&mut self, name: &str, value: Option<bool>) {
        if let Some(v) = value {
            self.add_bool(name, v);
        }
    }

    /// Add a number field.
    pub fn add_number(&mut self, name: &str, value: impl std::fmt::Display) {
        self.fields.push(format!(r#""{name}"":{value}"#));
    }

    /// Add an optional number field.
    pub fn add_optional_number<T: std::fmt::Display>(&mut self, name: &str, value: Option<T>) {
        if let Some(v) = value {
            self.add_number(name, v);
        }
    }

    /// Build the final JSON object string.
    pub fn build(self) -> String {
        format!("{{{}}}", self.fields.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_json_string() {
        assert_eq!(escape_json_string("hello"), "hello");
        assert_eq!(escape_json_string(r#"hello "world""#), r#"hello \"world\""#);
        assert_eq!(escape_json_string("line1\nline2"), r#"line1\nline2"#);
        assert_eq!(escape_json_string("\t\r\n"), r#"\t\r\n"#);
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b"hello"), "aGVsbG8=");
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
    }

    #[test]
    fn test_json_object_builder() {
        let mut obj = JsonObjectBuilder::new();
        obj.add_string("name", "test");
        obj.add_number("count", 42);
        obj.add_bool("enabled", true);

        let json = obj.build();
        assert!(json.contains(r#""name":"test""#));
        assert!(json.contains(r#""count":42"#));
        assert!(json.contains(r#""enabled":true"#));
    }
}