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

/// Streaming base64 encoder for bounded-memory encoding of large data.
///
/// Maintains internal state to handle data that doesn't align to 3-byte boundaries.
/// Base64 encodes 3 input bytes → 4 output bytes, so partial chunks are buffered.
///
/// # Example
/// ```ignore
/// let mut encoder = Base64StreamEncoder::new();
///
/// // Encode chunks as they arrive
/// let chunk1 = encoder.encode_chunk(&[0x48, 0x65]);  // "He" → buffered
/// let chunk2 = encoder.encode_chunk(&[0x6c, 0x6c, 0x6f]); // "llo" → "SGVs" + buffer "lo"
///
/// // Finalize adds padding if needed
/// let final_chunk = encoder.finalize();  // "bG8="
/// ```
pub struct Base64StreamEncoder {
    /// Buffer for incomplete 3-byte groups (0-2 bytes)
    buffer: [u8; 2],
    /// Number of bytes currently in buffer
    buffer_len: usize,
}

impl Base64StreamEncoder {
    /// Create a new streaming encoder.
    pub fn new() -> Self {
        Self {
            buffer: [0; 2],
            buffer_len: 0,
        }
    }

    /// Encode a chunk of data, returning base64 output for complete 3-byte groups.
    ///
    /// Incomplete bytes at the end are buffered for the next call or finalize().
    /// Returns empty string if all input is buffered.
    pub fn encode_chunk(&mut self, data: &[u8]) -> String {
        if data.is_empty() {
            return String::new();
        }

        // Calculate total bytes available (buffer + new data)
        let total_len = self.buffer_len + data.len();

        // Calculate how many complete 3-byte groups we can encode
        let complete_groups = total_len / 3;

        if complete_groups == 0 {
            // Not enough for even one group, buffer everything
            for (i, &byte) in data.iter().enumerate() {
                if self.buffer_len + i < 2 {
                    self.buffer[self.buffer_len + i] = byte;
                }
            }
            self.buffer_len += data.len();
            return String::new();
        }

        // Calculate how many bytes to encode
        let bytes_to_encode = complete_groups * 3;
        let bytes_from_data = bytes_to_encode - self.buffer_len;

        // Build input for encoding: buffer + portion of data
        let mut input = Vec::with_capacity(bytes_to_encode);
        input.extend_from_slice(&self.buffer[..self.buffer_len]);
        input.extend_from_slice(&data[..bytes_from_data]);

        // Encode complete groups
        let encoded = BASE64.encode(&input);

        // Buffer remaining bytes from data
        let remaining_bytes = &data[bytes_from_data..];
        self.buffer_len = remaining_bytes.len();
        self.buffer[..self.buffer_len].copy_from_slice(remaining_bytes);

        encoded
    }

    /// Finalize encoding, returning base64 output for any buffered bytes with padding.
    ///
    /// Consumes the encoder. If no bytes are buffered, returns empty string.
    pub fn finalize(self) -> String {
        if self.buffer_len == 0 {
            return String::new();
        }

        // Encode remaining buffered bytes (will add padding)
        BASE64.encode(&self.buffer[..self.buffer_len])
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    // === escape_json_string tests ===

    #[test]
    fn test_escape_json_string_no_escapes() {
        assert_eq!(escape_json_string("hello world"), "hello world");
    }

    #[test]
    fn test_escape_json_string_quotes() {
        assert_eq!(escape_json_string("say \"hello\""), r#"say \"hello\""#);
    }

    #[test]
    fn test_escape_json_string_backslash() {
        assert_eq!(escape_json_string(r"path\to\file"), r#"path\\to\\file"#);
    }

    #[test]
    fn test_escape_json_string_newline() {
        assert_eq!(escape_json_string("line1\nline2"), r"line1\nline2");
    }

    #[test]
    fn test_escape_json_string_carriage_return() {
        assert_eq!(escape_json_string("line1\rline2"), r"line1\rline2");
    }

    #[test]
    fn test_escape_json_string_tab() {
        assert_eq!(escape_json_string("col1\tcol2"), r"col1\tcol2");
    }

    #[test]
    fn test_escape_json_string_backspace() {
        assert_eq!(escape_json_string("test\u{0008}"), r"test\b");
    }

    #[test]
    fn test_escape_json_string_form_feed() {
        assert_eq!(escape_json_string("test\u{000C}"), r"test\f");
    }

    #[test]
    fn test_escape_json_string_control_char() {
        assert_eq!(escape_json_string("\u{0001}"), r"\u0001");
    }

    #[test]
    fn test_escape_json_string_no_embedded_newlines() {
        let input = "line1\nline2\rline3\r\nline4";
        let escaped = escape_json_string(input);
        assert!(!escaped.contains('\n'));
        assert!(!escaped.contains('\r'));
    }

    #[test]
    fn test_escape_json_string_unicode() {
        assert_eq!(escape_json_string("emoji: 🎯"), "emoji: 🎯");
    }

    // === compact_json tests ===

    #[test]
    fn test_compact_json_removes_whitespace() {
        let pretty = r#"{
            "type": "object",
            "properties": {
                "name": "value"
            }
        }"#;
        let compact = compact_json(pretty);
        assert!(!compact.contains('\n'));
        assert!(!compact.contains("  "));
        // Note: serde_json may reorder object keys, so we just verify structure
        assert!(compact.contains(r#""type":"object""#));
        assert!(compact.contains(r#""properties":{"name":"value"}"#));
    }

    #[test]
    fn test_compact_json_already_compact() {
        let input = r#"{"key":"value","number":42}"#;
        assert_eq!(compact_json(input), input);
    }

    #[test]
    fn test_compact_json_array() {
        let pretty = r#"[
            1,
            2,
            3
        ]"#;
        assert_eq!(compact_json(pretty), "[1,2,3]");
    }

    #[test]
    #[should_panic(expected = "Handler provided invalid JSON")]
    fn test_compact_json_invalid_json() {
        compact_json("not valid json");
    }

    #[test]
    #[should_panic(expected = "Handler provided invalid JSON")]
    fn test_compact_json_truncated_object() {
        compact_json(r#"{"key":"value""#);
    }

    // === Base64StreamEncoder tests ===

    #[test]
    fn test_base64_stream_encoder_aligned_3_bytes() {
        let mut encoder = Base64StreamEncoder::new();
        let chunk = encoder.encode_chunk(b"abc");
        assert_eq!(chunk, "YWJj");
        assert_eq!(encoder.finalize(), "");
    }

    #[test]
    fn test_base64_stream_encoder_aligned_6_bytes() {
        let mut encoder = Base64StreamEncoder::new();
        let chunk = encoder.encode_chunk(b"abcdef");
        assert_eq!(chunk, "YWJjZGVm");
        assert_eq!(encoder.finalize(), "");
    }

    #[test]
    fn test_base64_stream_encoder_1_byte_buffered() {
        let mut encoder = Base64StreamEncoder::new();

        // First chunk: 1 byte (buffered)
        assert_eq!(encoder.encode_chunk(b"a"), "");

        // Second chunk: 2 bytes (3 total, encodes)
        assert_eq!(encoder.encode_chunk(b"bc"), "YWJj");

        assert_eq!(encoder.finalize(), "");
    }

    #[test]
    fn test_base64_stream_encoder_2_bytes_buffered() {
        let mut encoder = Base64StreamEncoder::new();

        // First chunk: 2 bytes (buffered)
        assert_eq!(encoder.encode_chunk(b"ab"), "");

        // Second chunk: 1 byte (3 total, encodes)
        assert_eq!(encoder.encode_chunk(b"c"), "YWJj");

        assert_eq!(encoder.finalize(), "");
    }

    #[test]
    fn test_base64_stream_encoder_finalize_1_byte() {
        let mut encoder = Base64StreamEncoder::new();
        encoder.encode_chunk(b"abcd");  // Encodes "abc", buffers "d"

        let final_chunk = encoder.finalize();
        assert_eq!(final_chunk, "ZA==");  // "d" with padding
    }

    #[test]
    fn test_base64_stream_encoder_finalize_2_bytes() {
        let mut encoder = Base64StreamEncoder::new();
        encoder.encode_chunk(b"abcde");  // Encodes "abc", buffers "de"

        let final_chunk = encoder.finalize();
        assert_eq!(final_chunk, "ZGU=");  // "de" with padding
    }

    #[test]
    fn test_base64_stream_encoder_empty_finalize() {
        let encoder = Base64StreamEncoder::new();
        assert_eq!(encoder.finalize(), "");
    }

    #[test]
    fn test_base64_stream_encoder_empty_chunk() {
        let mut encoder = Base64StreamEncoder::new();
        assert_eq!(encoder.encode_chunk(b""), "");
        assert_eq!(encoder.finalize(), "");
    }

    #[test]
    fn test_base64_stream_encoder_multi_chunk_matches_single_pass() {
        let mut encoder = Base64StreamEncoder::new();

        let mut result = String::new();
        result.push_str(&encoder.encode_chunk(b"Hell"));
        result.push_str(&encoder.encode_chunk(b"o, Wo"));
        result.push_str(&encoder.encode_chunk(b"rld!"));
        result.push_str(&encoder.finalize());

        let expected = BASE64.encode(b"Hello, World!");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_base64_stream_encoder_single_byte_chunks() {
        let mut encoder = Base64StreamEncoder::new();

        let mut result = String::new();
        result.push_str(&encoder.encode_chunk(b"a"));
        result.push_str(&encoder.encode_chunk(b"b"));
        result.push_str(&encoder.encode_chunk(b"c"));
        result.push_str(&encoder.encode_chunk(b"d"));
        result.push_str(&encoder.finalize());

        let expected = BASE64.encode(b"abcd");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_base64_stream_encoder_large_data() {
        let mut encoder = Base64StreamEncoder::new();

        let data = b"The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs.";

        let mut result = String::new();
        for chunk in data.chunks(7) {  // Weird chunk size to test misalignment
            result.push_str(&encoder.encode_chunk(chunk));
        }
        result.push_str(&encoder.finalize());

        let expected = BASE64.encode(data);
        assert_eq!(result, expected);
    }

    // === JsonObjectBuilder tests ===

    #[test]
    fn test_json_object_builder_empty() {
        let obj = JsonObjectBuilder::new();
        assert_eq!(obj.build(), "{}");
    }

    #[test]
    fn test_json_object_builder_single_string() {
        let mut obj = JsonObjectBuilder::new();
        obj.add_string("name", "value");
        assert_eq!(obj.build(), r#"{"name":"value"}"#);
    }

    #[test]
    fn test_json_object_builder_string_with_escapes() {
        let mut obj = JsonObjectBuilder::new();
        obj.add_string("text", "line1\nline2");
        assert_eq!(obj.build(), r#"{"text":"line1\nline2"}"#);
    }

    #[test]
    fn test_json_object_builder_bool_true() {
        let mut obj = JsonObjectBuilder::new();
        obj.add_bool("flag", true);
        assert_eq!(obj.build(), r#"{"flag":true}"#);
    }

    #[test]
    fn test_json_object_builder_bool_false() {
        let mut obj = JsonObjectBuilder::new();
        obj.add_bool("flag", false);
        assert_eq!(obj.build(), r#"{"flag":false}"#);
    }

    #[test]
    fn test_json_object_builder_number_integer() {
        let mut obj = JsonObjectBuilder::new();
        obj.add_number("count", 42);
        assert_eq!(obj.build(), r#"{"count":42}"#);
    }

    #[test]
    fn test_json_object_builder_number_float() {
        let mut obj = JsonObjectBuilder::new();
        obj.add_number("value", 3.14);
        assert_eq!(obj.build(), r#"{"value":3.14}"#);
    }

    #[test]
    fn test_json_object_builder_raw_json_object() {
        let mut obj = JsonObjectBuilder::new();
        obj.add_raw_json("nested", r#"{"inner":"value"}"#);
        assert_eq!(obj.build(), r#"{"nested":{"inner":"value"}}"#);
    }

    #[test]
    fn test_json_object_builder_raw_json_array() {
        let mut obj = JsonObjectBuilder::new();
        obj.add_raw_json("items", r#"[1,2,3]"#);
        assert_eq!(obj.build(), r#"{"items":[1,2,3]}"#);
    }

    #[test]
    fn test_json_object_builder_validated_json() {
        let mut obj = JsonObjectBuilder::new();
        obj.add_validated_json("schema", "{\n  \"type\": \"object\"\n}");
        let result = obj.build();
        assert!(!result.contains('\n'));
        assert!(result.contains(r#""schema":{"type":"object"}"#));
    }

    #[test]
    fn test_json_object_builder_multiple_fields() {
        let mut obj = JsonObjectBuilder::new();
        obj.add_string("name", "test");
        obj.add_number("count", 5);
        obj.add_bool("active", true);
        obj.add_raw_json("data", r#"{"key":"val"}"#);

        let result = obj.build();

        // Parse as JSON to verify validity
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["name"], "test");
        assert_eq!(parsed["count"], 5);
        assert_eq!(parsed["active"], true);
        assert_eq!(parsed["data"]["key"], "val");
    }

    #[test]
    fn test_json_object_builder_field_ordering() {
        let mut obj = JsonObjectBuilder::new();
        obj.add_string("a", "first");
        obj.add_string("b", "second");
        obj.add_string("c", "third");

        let result = obj.build();

        // Field order should be preserved
        let a_pos = result.find(r#""a":"first""#).unwrap();
        let b_pos = result.find(r#""b":"second""#).unwrap();
        let c_pos = result.find(r#""c":"third""#).unwrap();

        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
    }

    #[test]
    fn test_json_object_builder_produces_valid_json() {
        let mut obj = JsonObjectBuilder::new();
        obj.add_string("string", "value");
        obj.add_number("number", 42);
        obj.add_bool("bool", true);
        obj.add_raw_json("null", "null");
        obj.add_raw_json("array", "[1,2,3]");
        obj.add_raw_json("object", r#"{"nested":true}"#);

        let result = obj.build();

        // Should parse as valid JSON
        serde_json::from_str::<serde_json::Value>(&result).unwrap();
    }
}
