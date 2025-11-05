//! Universal Server I/O Implementation
//!
//! Implements the server-io interface with transport-agnostic framing support.
//! Handles bidirectional JSON-RPC message exchange with runtime-specified framing.
//!
//! Architecture:
//! - Transport passes frame on each parse/send call (explicit, no state)
//! - Single parse_message() function replaces 4 parse_* functions
//! - Single send_message() function replaces 4 write_* functions
//! - No compile-time feature flags for transport selection
//!
//! This component provides full spec-compliant MCP 2025-06-18 message handling
//! for any transport (HTTP SSE, stdio, custom) via runtime framing parameters.

#![allow(warnings)]

mod bindings {
    wit_bindgen::generate!({
        world: "server-io",
        generate_all,
    });
}

mod parser;
mod serializer;
mod stream_reader;

#[cfg(test)]
mod tests;

use bindings::exports::wasmcp::mcp_v20250618::server_io::{
    Guest, IoError, MessageFrame, ReadLimit,
};
use bindings::wasi::io::streams::{InputStream, OutputStream, StreamError};
use bindings::wasmcp::mcp_v20250618::mcp::*;

use crate::stream_reader::StreamConfig;

struct ServerIo;

impl Guest for ServerIo {
    /// Parse an incoming message from the client
    ///
    /// Reads from the input stream according to the read limit, strips framing,
    /// and parses the JSON-RPC message into a client-message variant.
    fn parse_message(
        input: &InputStream,
        limit: ReadLimit,
        frame: MessageFrame,
    ) -> Result<ClientMessage, IoError> {
        // Read raw bytes based on limit
        let raw_bytes = match limit {
            ReadLimit::Delimiter(delim) => read_until_delimiter(input, &delim)?,
            ReadLimit::MaxBytes(max) => read_max_bytes(input, max)?,
        };

        // Strip framing prefix and suffix
        let json_bytes = strip_framing(&raw_bytes, &frame)?;

        // Convert to UTF-8 string
        let json_str = String::from_utf8(json_bytes)
            .map_err(|e| IoError::InvalidJsonrpc(format!("Invalid UTF-8: {}", e)))?;

        // Parse JSON
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| IoError::InvalidJsonrpc(format!("Invalid JSON: {}", e)))?;

        // Determine message type and parse
        parse_client_message(&json)
    }

    /// Send a message to the client
    ///
    /// Serializes the server-message variant to JSON-RPC format, applies framing,
    /// and writes to the output stream.
    fn send_message(
        output: &OutputStream,
        message: ServerMessage,
        frame: MessageFrame,
    ) -> Result<(), IoError> {
        // Get framed bytes
        let framed = serialize_message_to_bytes(message, &frame)?;

        eprintln!("[SERVER-IO] Framed message length: {} bytes", framed.len());

        // Write to stream
        write_bytes(output, &framed)?;

        eprintln!("[SERVER-IO] Message written and flushed");

        Ok(())
    }

    /// Flush buffered data to stream (for buffered mode)
    ///
    /// In buffered mode (MCP_SSE_BUFFER=true), all writes accumulate in memory.
    /// This function writes the entire buffer to the stream in one blocking operation.
    fn flush_buffer(output: &OutputStream) -> Result<(), IoError> {
        if !is_buffer_mode() {
            eprintln!("[SERVER-IO] Not in buffer mode, nothing to flush");
            return Ok(());
        }

        let data = BUFFER.with(|buf| {
            let borrowed = buf.borrow();
            borrowed.clone()
        });

        eprintln!(
            "[SERVER-IO] Flushing {} bytes via blocking_write_and_flush",
            data.len()
        );

        output.blocking_write_and_flush(&data).map_err(|e| {
            eprintln!("[SERVER-IO] blocking_write_and_flush failed: {:?}", e);
            IoError::Stream(e)
        })?;

        // Clear buffer after successful write
        BUFFER.with(|buf| {
            buf.borrow_mut().clear();
        });

        eprintln!("[SERVER-IO] Buffer flushed successfully");
        Ok(())
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Serialize a message to framed bytes WITHOUT writing to stream
///
/// This is exported for transport layer to use in buffered mode.
/// Returns the complete framed message ready to write.
fn serialize_message_to_bytes(
    message: ServerMessage,
    frame: &MessageFrame,
) -> Result<Vec<u8>, IoError> {
    // Serialize message to JSON-RPC
    let json_rpc = serialize_server_message(&message)?;

    // Convert to string
    let json_str = json_rpc.to_string();

    eprintln!(
        "[SERVER-IO] Serializing message to bytes: {}",
        &json_str[..json_str.len().min(100)]
    );

    // Apply framing
    let mut framed = Vec::new();
    framed.extend_from_slice(&frame.prefix);
    framed.extend_from_slice(json_str.as_bytes());
    framed.extend_from_slice(&frame.suffix);

    Ok(framed)
}

// =============================================================================
// READING FUNCTIONS
// =============================================================================

/// Read from stream until delimiter is found
///
/// Uses optimized fast path for single-byte delimiters (stdio),
/// and boundary-safe search for multi-byte delimiters (SSE).
fn read_until_delimiter(stream: &InputStream, delimiter: &[u8]) -> Result<Vec<u8>, IoError> {
    if delimiter.len() == 1 {
        // Fast path: single-byte delimiter (e.g., stdio '\n')
        read_until_byte(stream, delimiter[0])
    } else {
        // Generic path: multi-byte delimiter with boundary handling (e.g., SSE '\n\n')
        read_until_multibyte_delimiter(stream, delimiter)
    }
}

/// Fast path for single-byte delimiters
///
/// Iterates byte-by-byte, naturally handling chunk boundaries.
fn read_until_byte(stream: &InputStream, delimiter: u8) -> Result<Vec<u8>, IoError> {
    const MAX_SIZE: usize = 10 * 1024 * 1024; // 10MB max
    const CHUNK_SIZE: usize = 4096; // Read 4KB chunks
    let mut buffer = Vec::new();

    loop {
        if buffer.len() >= MAX_SIZE {
            return Err(IoError::Unexpected(format!(
                "Message exceeds maximum size of {} bytes",
                MAX_SIZE
            )));
        }

        let chunk = stream
            .read(CHUNK_SIZE as u64)
            .map_err(|e| IoError::Stream(e))?;

        if chunk.is_empty() {
            if buffer.is_empty() {
                return Err(IoError::Unexpected(
                    "Stream closed before delimiter".to_string(),
                ));
            } else {
                break; // EOF reached
            }
        }

        // Scan byte-by-byte for delimiter
        for byte in chunk {
            if byte == delimiter {
                // Found delimiter - return buffer without it
                return Ok(buffer);
            }
            buffer.push(byte);
        }
    }

    Ok(buffer)
}

/// Generic path for multi-byte delimiters with boundary handling
///
/// Searches across chunk boundaries by checking overlapping regions.
/// This prevents missing delimiters split across two read chunks.
///
/// Example: For delimiter "\n\n" with chunks ending at boundary:
///   Chunk 1: "data: {...}\n"
///   Chunk 2: "\ndata: {...}"
///
/// Without boundary handling, each chunk search would fail (only single \n).
/// With boundary handling, we search the overlapping region and find the delimiter.
fn read_until_multibyte_delimiter(
    stream: &InputStream,
    delimiter: &[u8],
) -> Result<Vec<u8>, IoError> {
    const MAX_SIZE: usize = 10 * 1024 * 1024; // 10MB max
    const CHUNK_SIZE: usize = 4096; // Read 4KB chunks
    let mut buffer = Vec::new();

    loop {
        if buffer.len() >= MAX_SIZE {
            return Err(IoError::Unexpected(format!(
                "Message exceeds maximum size of {} bytes",
                MAX_SIZE
            )));
        }

        let chunk = stream
            .read(CHUNK_SIZE as u64)
            .map_err(|e| IoError::Stream(e))?;

        if chunk.is_empty() {
            if buffer.is_empty() {
                return Err(IoError::Unexpected(
                    "Stream closed before delimiter".to_string(),
                ));
            } else {
                break; // EOF reached
            }
        }

        let chunk_len = chunk.len();
        buffer.extend_from_slice(&chunk);

        // Use boundary-aware search (tested in unit tests)
        if let Some((pos, _found)) = search_with_boundary(&buffer, chunk_len, delimiter) {
            buffer.truncate(pos);
            return Ok(buffer);
        }
    }

    Ok(buffer)
}

/// Read up to max_bytes from stream
fn read_max_bytes(stream: &InputStream, max_bytes: u64) -> Result<Vec<u8>, IoError> {
    let config = StreamConfig {
        max_size: max_bytes,
        chunk_size: 4096,
    };

    stream_reader::read_bytes_chunked(stream, &config).map_err(|e| IoError::Unexpected(e))
}

/// Find position of needle in haystack
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }

    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Search for delimiter in buffer with boundary-aware logic
///
/// This function handles the case where a delimiter might be split across
/// chunk boundaries by searching an overlapping region.
///
/// Returns Some((position, found)) where:
/// - position: byte offset of delimiter start in buffer
/// - found: true if delimiter found, false if need more data
///
/// # Arguments
/// * `buffer` - Complete buffer accumulated so far
/// * `new_chunk_len` - Length of the most recently added chunk
/// * `delimiter` - Delimiter bytes to search for
///
/// # Example
/// ```ignore
/// // Buffer: "data: {...}\n" + "\nmore data"
/// //                      ^~^  delimiter "\n\n" spans boundary
/// let buffer = b"data: {...}\n\nmore data";
/// let result = search_with_boundary(buffer, 10, b"\n\n");
/// assert_eq!(result, Some((12, true))); // Found at position 12
/// ```
fn search_with_boundary(
    buffer: &[u8],
    new_chunk_len: usize,
    delimiter: &[u8],
) -> Option<(usize, bool)> {
    if buffer.is_empty() || delimiter.is_empty() {
        return None;
    }

    let buffer_len = buffer.len();
    let delimiter_len = delimiter.len();

    // Calculate overlapping search region
    // We need to check the last (delimiter_len - 1) bytes of previous data
    // plus all of the new chunk, to catch delimiters spanning the boundary
    let search_start = buffer_len.saturating_sub(new_chunk_len + delimiter_len - 1);

    // Search in the overlapping region
    if let Some(relative_pos) = find_subsequence(&buffer[search_start..], delimiter) {
        let absolute_pos = search_start + relative_pos;
        Some((absolute_pos, true))
    } else {
        None
    }
}

// =============================================================================
// TESTS - Boundary-aware delimiter search
// =============================================================================

#[cfg(all(test, not(target_family = "wasm")))]
mod boundary_tests {
    use super::*;

    #[test]
    fn test_find_subsequence_basic() {
        assert_eq!(find_subsequence(b"hello world", b"world"), Some(6));
        assert_eq!(find_subsequence(b"hello world", b"xyz"), None);
        assert_eq!(find_subsequence(b"", b"test"), None);
        assert_eq!(find_subsequence(b"test", b""), Some(0));
    }

    #[test]
    fn test_search_with_boundary_no_split() {
        // Delimiter entirely within new chunk
        let buffer = b"data: {\"test\":\"value\"}\n\n";
        let result = search_with_boundary(buffer, 10, b"\n\n");
        assert!(result.is_some());
        let (pos, found) = result.unwrap();
        assert!(found);
        assert_eq!(&buffer[pos..pos + 2], b"\n\n");
    }

    #[test]
    fn test_search_with_boundary_split_delimiter() {
        // Delimiter split across boundary: first chunk ends with \n, second starts with \n
        let mut buffer = Vec::new();
        buffer.extend_from_slice(b"data: {\"test\":\"value\"}\n"); // First chunk
        let chunk1_len = buffer.len();
        buffer.extend_from_slice(b"\nmore data"); // Second chunk adds the second \n

        let result = search_with_boundary(&buffer, b"\nmore data".len(), b"\n\n");
        assert!(result.is_some());
        let (pos, found) = result.unwrap();
        assert!(found);
        assert_eq!(pos, chunk1_len - 1); // Delimiter starts at last byte of first chunk
        assert_eq!(&buffer[pos..pos + 2], b"\n\n");
    }

    #[test]
    fn test_search_with_boundary_single_byte_delimiter() {
        let buffer = b"hello\nworld";
        let result = search_with_boundary(buffer, 6, b"\n");
        assert!(result.is_some());
        let (pos, found) = result.unwrap();
        assert!(found);
        assert_eq!(pos, 5);
    }

    #[test]
    fn test_search_with_boundary_delimiter_at_exact_boundary() {
        // Chunk 1: "test"
        // Chunk 2: "\n\ndata"
        // Delimiter "\n\n" starts exactly at boundary
        let buffer = b"test\n\ndata";
        let result = search_with_boundary(buffer, 6, b"\n\n"); // "\n\ndata" is new chunk
        assert!(result.is_some());
        let (pos, found) = result.unwrap();
        assert!(found);
        assert_eq!(pos, 4);
        assert_eq!(&buffer[pos..pos + 2], b"\n\n");
    }

    #[test]
    fn test_search_with_boundary_no_delimiter_found() {
        let buffer = b"data without delimiter";
        let result = search_with_boundary(buffer, 10, b"\n\n");
        assert!(result.is_none());
    }

    #[test]
    fn test_search_with_boundary_three_byte_delimiter() {
        // Test with unusual multi-byte delimiter
        let mut buffer = Vec::new();
        buffer.extend_from_slice(b"prefix-AB"); // First chunk ends with "AB"
        buffer.extend_from_slice(b"C-suffix"); // Second chunk starts with "C"

        let result = search_with_boundary(&buffer, 8, b"ABC");
        assert!(result.is_some());
        let (pos, found) = result.unwrap();
        assert!(found);
        assert_eq!(&buffer[pos..pos + 3], b"ABC");
    }

    #[test]
    fn test_search_with_boundary_delimiter_split_three_ways() {
        // Pathological case: delimiter "ABC" split as "A" | "B" | "C" across three chunks
        // This function is called incrementally, so we test two-chunk boundaries
        let mut buffer = Vec::new();
        buffer.extend_from_slice(b"prefix-A"); // First chunk
        buffer.extend_from_slice(b"BC-suffix"); // Second chunk

        let result = search_with_boundary(&buffer, 9, b"ABC");
        assert!(result.is_some());
        let (pos, found) = result.unwrap();
        assert!(found);
        assert_eq!(&buffer[pos..pos + 3], b"ABC");
    }

    #[test]
    fn test_search_with_boundary_empty_buffer() {
        let buffer = b"";
        let result = search_with_boundary(buffer, 0, b"\n\n");
        assert!(result.is_none());
    }

    #[test]
    fn test_search_with_boundary_overlapping_search_region() {
        // Test that we search far enough back
        // Buffer: "xxxx" + "x\n\n" - delimiter at boundary
        let buffer = b"xxxxx\n\n";
        let result = search_with_boundary(buffer, 3, b"\n\n"); // Last 3 bytes: "x\n\n"
        assert!(result.is_some());
        let (pos, found) = result.unwrap();
        assert!(found);
        assert_eq!(pos, 5);
    }
}

/// Strip framing prefix and suffix from raw bytes
fn strip_framing(raw: &[u8], frame: &MessageFrame) -> Result<Vec<u8>, IoError> {
    let mut data = raw;

    // Strip prefix
    if !frame.prefix.is_empty() {
        if data.starts_with(&frame.prefix) {
            data = &data[frame.prefix.len()..];
        } else {
            return Err(IoError::InvalidJsonrpc(format!(
                "Message does not start with expected prefix: {:?}",
                String::from_utf8_lossy(&frame.prefix)
            )));
        }
    }

    // Strip suffix
    if !frame.suffix.is_empty() {
        if data.ends_with(&frame.suffix) {
            data = &data[..data.len() - frame.suffix.len()];
        } else {
            return Err(IoError::InvalidJsonrpc(format!(
                "Message does not end with expected suffix: {:?}",
                String::from_utf8_lossy(&frame.suffix)
            )));
        }
    }

    Ok(data.to_vec())
}

// =============================================================================
// WRITING FUNCTIONS
// =============================================================================

use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::new());
}

/// Check if we're in buffer mode
fn is_buffer_mode() -> bool {
    std::env::var("MCP_SSE_BUFFER")
        .ok()
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(true) // Default to buffered
}

/// Write bytes to output stream with backpressure handling
fn write_bytes(stream: &OutputStream, data: &[u8]) -> Result<(), IoError> {
    // Check if we're in buffer mode
    if is_buffer_mode() {
        eprintln!("[SERVER-IO] Buffer mode: accumulating {} bytes", data.len());
        BUFFER.with(|buf| {
            buf.borrow_mut().extend_from_slice(data);
        });
        return Ok(());
    }

    // Streaming mode: write immediately
    let mut offset = 0;
    eprintln!(
        "[SERVER-IO] write_bytes: total {} bytes to write",
        data.len()
    );

    while offset < data.len() {
        match stream.check_write() {
            Ok(0) => {
                return Err(IoError::Unexpected(format!(
                    "Write budget exhausted in streaming mode. Set MCP_SSE_BUFFER=true for buffered mode."
                )));
            }
            Ok(budget) => {
                let chunk_size = (data.len() - offset).min(budget as usize);
                eprintln!(
                    "[SERVER-IO] check_write returned budget {}, writing {} bytes",
                    budget, chunk_size
                );
                stream
                    .write(&data[offset..offset + chunk_size])
                    .map_err(|e| {
                        eprintln!("[SERVER-IO] write() failed: {:?}", e);
                        IoError::Stream(e)
                    })?;
                offset += chunk_size;
                eprintln!(
                    "[SERVER-IO] wrote chunk, offset now {}/{}",
                    offset,
                    data.len()
                );
                // Don't flush per-chunk - only at end
            }
            Err(e) => {
                eprintln!("[SERVER-IO] check_write() failed: {:?}", e);
                return Err(IoError::Stream(e));
            }
        }
    }

    // DON'T flush here - let buffer accumulate across multiple messages
    // Only the transport layer will flush at the very end via OutgoingBody::finish()
    eprintln!(
        "[SERVER-IO] Successfully wrote all {} bytes (no flush - buffered)",
        data.len()
    );
    Ok(())
}

// =============================================================================
// MESSAGE PARSING
// =============================================================================

/// Parse JSON into client-message variant
fn parse_client_message(json: &serde_json::Value) -> Result<ClientMessage, IoError> {
    // Check for "id" field to distinguish requests from notifications
    let has_id = json.get("id").is_some();

    // Check for "result" or "error" field to distinguish responses from requests
    let has_result = json.get("result").is_some();
    let has_error = json.get("error").is_some();

    if has_result {
        // This is a result response
        let id = json
            .get("id")
            .ok_or_else(|| IoError::InvalidMcp("Missing 'id' field in result".to_string()))?;
        let request_id = parser::parse_request_id(id)?;
        let client_result = parser::parse_client_result(json)?;
        Ok(ClientMessage::Result((request_id, client_result)))
    } else if has_error {
        // This is an error response
        let id = json.get("id").and_then(|id| {
            if id.is_null() {
                None
            } else {
                parser::parse_request_id(id).ok()
            }
        });
        let error = parser::parse_error(json)?;
        Ok(ClientMessage::Error((id, error)))
    } else if has_id {
        // This is a request
        let id = json.get("id").unwrap(); // We know it exists
        let request_id = parser::parse_request_id(id)?;
        let client_request = parser::parse_client_request(json)?;
        Ok(ClientMessage::Request((request_id, client_request)))
    } else {
        // This is a notification (no id field)
        let client_notification = parser::parse_client_notification(json)?;
        Ok(ClientMessage::Notification(client_notification))
    }
}

// =============================================================================
// MESSAGE SERIALIZATION
// =============================================================================

/// Serialize server-message variant to JSON-RPC
fn serialize_server_message(message: &ServerMessage) -> Result<serde_json::Value, IoError> {
    match message {
        ServerMessage::Request((id, request)) => {
            // Generate JSON-RPC request
            let (method, params) = serialize_server_request(request);
            Ok(serde_json::json!({
                "jsonrpc": "2.0",
                "id": serialize_request_id(id),
                "method": method,
                "params": params
            }))
        }
        ServerMessage::Result((id, result)) => {
            // Generate JSON-RPC result response
            Ok(serializer::serialize_jsonrpc_response(id, Ok(result)))
        }
        ServerMessage::Error((id, error_code)) => {
            // Extract Error record from ErrorCode variant
            use ErrorCode::*;
            let error = match error_code {
                ParseError(e) | InvalidRequest(e) | MethodNotFound(e) | InvalidParams(e)
                | InternalError(e) | Server(e) | JsonRpc(e) | Mcp(e) => e,
            };

            // Generate JSON-RPC error response
            let mut error_obj = serde_json::Map::new();
            error_obj.insert("code".to_string(), serde_json::json!(error.code));
            error_obj.insert(
                "message".to_string(),
                serde_json::Value::String(error.message.clone()),
            );
            if let Some(ref data) = error.data {
                if let Ok(data_value) = serde_json::from_str::<serde_json::Value>(data) {
                    error_obj.insert("data".to_string(), data_value);
                }
            }

            Ok(serde_json::json!({
                "jsonrpc": "2.0",
                "id": id.as_ref().map(serialize_request_id),
                "error": error_obj
            }))
        }
        ServerMessage::Notification(notification) => {
            // Generate JSON-RPC notification
            let (method, params) = serialize_server_notification(notification);
            Ok(serde_json::json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": params
            }))
        }
    }
}

// =============================================================================
// SERIALIZATION HELPERS
// =============================================================================

fn serialize_request_id(id: &RequestId) -> serde_json::Value {
    match id {
        RequestId::String(s) => serde_json::Value::String(s.clone()),
        RequestId::Number(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
    }
}

fn serialize_server_notification(
    notification: &ServerNotification,
) -> (&'static str, serde_json::Value) {
    match notification {
        ServerNotification::ToolsListChanged(opts) => (
            "notifications/tools/list_changed",
            serialize_notification_options(opts),
        ),
        ServerNotification::ResourcesListChanged(opts) => (
            "notifications/resources/list_changed",
            serialize_notification_options(opts),
        ),
        ServerNotification::ResourcesUpdated(resource_updated) => {
            let mut params = serde_json::Map::new();
            params.insert(
                "uri".to_string(),
                serde_json::Value::String(resource_updated.uri.clone()),
            );

            if let Some(ref meta) = resource_updated.meta {
                if let Ok(meta_value) = serde_json::from_str::<serde_json::Value>(meta) {
                    params.insert("_meta".to_string(), meta_value);
                }
            }

            (
                "notifications/resources/updated",
                serde_json::Value::Object(params),
            )
        }
        ServerNotification::PromptsListChanged(opts) => (
            "notifications/prompts/list_changed",
            serialize_notification_options(opts),
        ),
        ServerNotification::Log(log_msg) => (
            "notifications/message",
            serde_json::json!({
                "level": log_level_to_string(&log_msg.level),
                "logger": log_msg.logger,
                "data": log_msg.data,
            }),
        ),
        ServerNotification::Cancellation(cancelled) => (
            "notifications/cancelled",
            serde_json::json!({
                "requestId": serialize_request_id(&cancelled.request_id),
                "reason": cancelled.reason,
            }),
        ),
        ServerNotification::Progress(progress) => {
            let progress_token_value = match &progress.progress_token {
                ProgressToken::String(s) => serde_json::Value::String(s.clone()),
                ProgressToken::Integer(i) => {
                    serde_json::Value::Number(serde_json::Number::from(*i))
                }
            };

            let mut params = serde_json::Map::new();
            params.insert("progressToken".to_string(), progress_token_value);
            params.insert("progress".to_string(), serde_json::json!(progress.progress));

            if let Some(ref t) = progress.total {
                params.insert("total".to_string(), serde_json::json!(t));
            }

            if let Some(ref m) = progress.message {
                params.insert("message".to_string(), serde_json::Value::String(m.clone()));
            }

            ("notifications/progress", serde_json::Value::Object(params))
        }
    }
}

fn serialize_notification_options(opts: &NotificationOptions) -> serde_json::Value {
    let mut params = serde_json::Map::new();

    // Add _meta field if present
    if let Some(ref meta) = opts.meta {
        if let Ok(meta_value) = serde_json::from_str::<serde_json::Value>(meta) {
            params.insert("_meta".to_string(), meta_value);
        }
    }

    // Unpack extras as arbitrary key-value pairs at the root params level
    if let Some(ref extras) = opts.extras {
        if let Ok(serde_json::Value::Object(extras_obj)) =
            serde_json::from_str::<serde_json::Value>(extras)
        {
            for (key, value) in extras_obj {
                params.insert(key, value);
            }
        }
    }

    serde_json::Value::Object(params)
}

fn serialize_server_request(request: &ServerRequest) -> (&'static str, serde_json::Value) {
    match request {
        ServerRequest::ElicitationCreate(elicit_req) => (
            "elicitation/create",
            serde_json::json!({
                "message": elicit_req.message,
                "requestedSchema": serialize_requested_schema(&elicit_req.requested_schema),
            }),
        ),
        ServerRequest::RootsList(roots_req) => {
            let mut params = serde_json::Map::new();
            if let Some(ref token) = roots_req.progress_token {
                let token_value = match token {
                    ProgressToken::String(s) => serde_json::Value::String(s.clone()),
                    ProgressToken::Integer(i) => {
                        serde_json::Value::Number(serde_json::Number::from(*i))
                    }
                };
                params.insert("progressToken".to_string(), token_value);
            }
            ("roots/list", serde_json::Value::Object(params))
        }
        ServerRequest::SamplingCreateMessage(sampling_req) => (
            "sampling/createMessage",
            serde_json::json!({
                "messages": sampling_req.messages.iter().map(serialize_sampling_message).collect::<Vec<_>>(),
                "modelPreferences": sampling_req.model_preferences.as_ref().map(serialize_model_preferences),
                "systemPrompt": sampling_req.system_prompt,
                "includeContext": serialize_include_context(&sampling_req.include_context),
                "temperature": sampling_req.temperature,
                "maxTokens": sampling_req.max_tokens,
                "stopSequences": sampling_req.stop_sequences,
                "metadata": sampling_req.metadata.as_ref().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()),
            }),
        ),
        ServerRequest::Ping(ping_req) => {
            let mut params = serde_json::Map::new();
            if let Some(ref token) = ping_req.progress_token {
                let token_value = match token {
                    ProgressToken::String(s) => serde_json::Value::String(s.clone()),
                    ProgressToken::Integer(i) => {
                        serde_json::Value::Number(serde_json::Number::from(*i))
                    }
                };
                params.insert("progressToken".to_string(), token_value);
            }
            if let Some(ref meta) = ping_req.meta {
                if let Ok(meta_value) = serde_json::from_str::<serde_json::Value>(meta) {
                    params.insert("_meta".to_string(), meta_value);
                }
            }
            for (k, v) in &ping_req.extras {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(v) {
                    params.insert(k.clone(), value);
                }
            }
            ("ping", serde_json::Value::Object(params))
        }
    }
}

fn serialize_requested_schema(schema: &RequestedSchema) -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": schema.properties.iter().map(|(k, v)| {
            (k.clone(), serialize_primitive_schema(v))
        }).collect::<serde_json::Map<_, _>>(),
        "required": schema.required,
    })
}

fn serialize_primitive_schema(schema: &PrimitiveSchemaDefinition) -> serde_json::Value {
    match schema {
        PrimitiveSchemaDefinition::StringSchema(s) => {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "type".to_string(),
                serde_json::Value::String("string".to_string()),
            );
            if let Some(ref desc) = s.description {
                obj.insert(
                    "description".to_string(),
                    serde_json::Value::String(desc.clone()),
                );
            }
            if let Some(ref title) = s.title {
                obj.insert(
                    "title".to_string(),
                    serde_json::Value::String(title.clone()),
                );
            }
            if let Some(ref format) = s.format {
                let format_str = match format {
                    StringSchemaFormat::Uri => "uri",
                    StringSchemaFormat::Email => "email",
                    StringSchemaFormat::Date => "date",
                    StringSchemaFormat::DateTime => "date-time",
                };
                obj.insert(
                    "format".to_string(),
                    serde_json::Value::String(format_str.to_string()),
                );
            }
            if let Some(min_len) = s.min_length {
                obj.insert("minLength".to_string(), serde_json::json!(min_len));
            }
            if let Some(max_len) = s.max_length {
                obj.insert("maxLength".to_string(), serde_json::json!(max_len));
            }
            serde_json::Value::Object(obj)
        }
        PrimitiveSchemaDefinition::NumberSchema(n) => {
            let mut obj = serde_json::Map::new();
            let type_str = match n.type_ {
                NumberSchemaType::Number => "number",
                NumberSchemaType::Integer => "integer",
            };
            obj.insert(
                "type".to_string(),
                serde_json::Value::String(type_str.to_string()),
            );
            if let Some(ref desc) = n.description {
                obj.insert(
                    "description".to_string(),
                    serde_json::Value::String(desc.clone()),
                );
            }
            if let Some(ref title) = n.title {
                obj.insert(
                    "title".to_string(),
                    serde_json::Value::String(title.clone()),
                );
            }
            if let Some(minimum) = n.minimum {
                obj.insert("minimum".to_string(), serde_json::json!(minimum));
            }
            if let Some(maximum) = n.maximum {
                obj.insert("maximum".to_string(), serde_json::json!(maximum));
            }
            serde_json::Value::Object(obj)
        }
        PrimitiveSchemaDefinition::BooleanSchema(b) => {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "type".to_string(),
                serde_json::Value::String("boolean".to_string()),
            );
            if let Some(ref desc) = b.description {
                obj.insert(
                    "description".to_string(),
                    serde_json::Value::String(desc.clone()),
                );
            }
            if let Some(ref title) = b.title {
                obj.insert(
                    "title".to_string(),
                    serde_json::Value::String(title.clone()),
                );
            }
            if let Some(default) = b.default {
                obj.insert("default".to_string(), serde_json::json!(default));
            }
            serde_json::Value::Object(obj)
        }
        PrimitiveSchemaDefinition::EnumSchema(e) => {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "enum".to_string(),
                serde_json::Value::Array(
                    e.enum_
                        .iter()
                        .map(|s| serde_json::Value::String(s.clone()))
                        .collect(),
                ),
            );
            if let Some(ref desc) = e.description {
                obj.insert(
                    "description".to_string(),
                    serde_json::Value::String(desc.clone()),
                );
            }
            if let Some(ref title) = e.title {
                obj.insert(
                    "title".to_string(),
                    serde_json::Value::String(title.clone()),
                );
            }
            if let Some(ref enum_names) = e.enum_names {
                obj.insert(
                    "enumNames".to_string(),
                    serde_json::Value::Array(
                        enum_names
                            .iter()
                            .map(|s| serde_json::Value::String(s.clone()))
                            .collect(),
                    ),
                );
            }
            serde_json::Value::Object(obj)
        }
    }
}

fn serialize_sampling_message(msg: &SamplingMessage) -> serde_json::Value {
    serde_json::json!({
        "role": match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        },
        "content": serialize_content_block(&msg.content)
    })
}

fn serialize_content_block(content: &ContentBlock) -> serde_json::Value {
    // Use serializer module for full content block serialization
    // This handles all content types including streams
    match serializer::convert_content_block(content) {
        Ok(json_block) => serde_json::to_value(json_block).unwrap_or_else(|e| {
            serde_json::json!({
                "type": "text",
                "text": format!("[error serializing content: {}]", e)
            })
        }),
        Err(e) => serde_json::json!({
            "type": "text",
            "text": format!("[error converting content: {}]", e)
        }),
    }
}

fn serialize_model_preferences(prefs: &ModelPreferences) -> serde_json::Value {
    serde_json::json!({
        "hints": prefs.hints.as_ref().map(|hints| {
            hints.iter().map(|h| serde_json::json!({
                "name": h.name,
                "extra": h.extra.as_ref().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()),
            })).collect::<Vec<_>>()
        }),
        "costPriority": prefs.cost_priority,
        "speedPriority": prefs.speed_priority,
        "intelligencePriority": prefs.intelligence_priority,
    })
}

fn serialize_include_context(ctx: &IncludeContext) -> &'static str {
    match ctx {
        IncludeContext::None => "none",
        IncludeContext::ThisServer => "thisServer",
        IncludeContext::AllServers => "allServers",
    }
}

fn log_level_to_string(level: &LogLevel) -> &'static str {
    match level {
        LogLevel::Debug => "debug",
        LogLevel::Info => "info",
        LogLevel::Notice => "notice",
        LogLevel::Warning => "warning",
        LogLevel::Error => "error",
        LogLevel::Critical => "critical",
        LogLevel::Alert => "alert",
        LogLevel::Emergency => "emergency",
    }
}

// Re-export internal modules for use by serializer
use serializer::convert_content_block;

bindings::export!(ServerIo with_types_in bindings);
