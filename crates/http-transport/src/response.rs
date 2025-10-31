//! HTTP response utilities for MCP transport
//!
//! Provides utilities for:
//! - Writing SSE (Server-Sent Events) responses
//! - Creating error responses with proper status codes
//! - Chunked writing that respects stream budgets
//! - Reading request bodies

use crate::bindings::wasi::http::types::{Fields, IncomingBody, OutgoingBody, OutgoingResponse};
use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use crate::bindings::wasmcp::mcp_v20250618::mcp::{ErrorCode, ServerResult};
use crate::bindings::wasmcp::mcp_v20250618::server_handler::RequestId;
use crate::{parser, serializer};

/// Write MCP response as Server-Sent Event
///
/// Per MCP spec: Responses MUST be sent as Server-Sent Events over the response stream.
/// https://modelcontextprotocol.io/specification/2025-06-18/basic/transports#sending-responses-from-the-server
///
/// # Arguments
/// * `output_stream` - Output stream to write SSE to
/// * `request_id` - Request ID from client request
/// * `result` - Server result (success or error)
///
/// # Returns
/// * `Ok(())` - Response written successfully
/// * `Err(String)` - If stream write fails
pub fn write_sse_response(
    output_stream: &OutputStream,
    request_id: RequestId,
    result: Result<ServerResult, ErrorCode>,
) -> Result<(), String> {
    // Serialize to JSON-RPC
    let json_rpc = serializer::serialize_jsonrpc_response(&request_id, result.as_ref());

    // Format as SSE event
    let event_data = serializer::format_sse_event(&json_rpc);

    // Write using check_write() to respect budget
    write_chunked(output_stream, event_data.as_bytes())
}

/// Write JSON-RPC response as plain JSON (not SSE)
///
/// Used for transport-level methods (initialize, ping, logging/setLevel)
/// that return plain JSON instead of Server-Sent Events.
///
/// # Arguments
/// * `request_id` - Request ID from client request
/// * `result` - Result value to serialize (use `serde_json::json!({})` for empty result)
/// * `additional_headers` - Optional additional headers (e.g., Mcp-Session-Id)
///
/// # Returns
/// * `Ok(OutgoingResponse)` - HTTP response with JSON body
/// * `Err(String)` - If response creation or writing fails
pub fn write_json_rpc_response(
    request_id: &RequestId,
    result: serde_json::Value,
    additional_headers: Option<&Fields>,
) -> Result<OutgoingResponse, String> {
    // Create headers with content-type
    let headers = Fields::new();
    headers
        .set("content-type", &[b"application/json".to_vec()])
        .map_err(|_| "Failed to set content-type")?;

    // Add any additional headers (e.g., Mcp-Session-Id)
    if let Some(extra_headers) = additional_headers {
        // Copy all headers from extra_headers to headers
        for header_name in extra_headers.entries() {
            let values = extra_headers.get(&header_name.0);
            if !values.is_empty() {
                headers
                    .set(&header_name.0, &values)
                    .map_err(|_| format!("Failed to set header: {}", header_name.0))?;
            }
        }
    }

    // Create response
    let response = OutgoingResponse::new(headers);
    response
        .set_status_code(200)
        .map_err(|_| "Failed to set status")?;

    // Get body and output stream
    let body = response.body().map_err(|_| "Failed to get response body")?;
    let output_stream = body.write().map_err(|_| "Failed to get output stream")?;

    // Build JSON-RPC response
    let json_result = serde_json::json!({
        "jsonrpc": "2.0",
        "id": match request_id {
            RequestId::Number(n) => serde_json::Value::Number(serde_json::Number::from(*n)),
            RequestId::String(s) => serde_json::Value::String(s.clone()),
        },
        "result": result
    });

    // Serialize and write
    let json_str = serde_json::to_string(&json_result)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    write_chunked(&output_stream, json_str.as_bytes())?;

    // Finish body
    drop(output_stream);
    OutgoingBody::finish(body, None).map_err(|_| "Failed to finish body")?;

    Ok(response)
}

/// Write bytes to output stream respecting budget
///
/// Uses check_write() to query available write budget and writes
/// only what the budget allows. If budget is exhausted, stops writing silently.
///
/// # Arguments
/// * `output_stream` - Output stream to write to
/// * `bytes` - Bytes to write
///
/// # Returns
/// * `Ok(())` - All bytes written or budget exhausted
/// * `Err(String)` - If stream write fails
pub fn write_chunked(output_stream: &OutputStream, bytes: &[u8]) -> Result<(), String> {
    // Write using check_write() to respect budget
    let mut offset = 0;
    while offset < bytes.len() {
        match output_stream.check_write() {
            Ok(0) => {
                // No budget available - stop writing silently
                break;
            }
            Ok(budget) => {
                // Write only what the budget allows
                let chunk_size = (bytes.len() - offset).min(budget as usize);
                let chunk = &bytes[offset..offset + chunk_size];
                output_stream.write(chunk).map_err(|e| match e {
                    StreamError::LastOperationFailed(_) => "Stream write failed".to_string(),
                    StreamError::Closed => "Stream closed".to_string(),
                })?;
                offset += chunk_size;
            }
            Err(e) => {
                return Err(match e {
                    StreamError::LastOperationFailed(_) => "Stream check failed".to_string(),
                    StreamError::Closed => "Stream closed".to_string(),
                });
            }
        }
    }

    Ok(())
}

/// Create HTTP error response
///
/// Supports special error format: "HTTP/404:Message" to specify status code.
/// Falls back to 400 Bad Request for generic errors.
///
/// Per MCP spec: "The HTTP response body MAY comprise a JSON-RPC error response that has no id"
///
/// # Arguments
/// * `error` - Error message (optionally prefixed with "HTTP/{code}:")
///
/// # Returns
/// OutgoingResponse with appropriate status code and JSON-RPC error body
pub fn create_error_response(error: String) -> OutgoingResponse {
    // Parse HTTP status code from error message if present
    // Format: "HTTP/404:Message" or just "Message"
    let (status_code, message) = if error.starts_with("HTTP/") {
        let parts: Vec<&str> = error.splitn(2, ':').collect();
        if parts.len() == 2 {
            let code_part = parts[0].trim_start_matches("HTTP/");
            let code = code_part.parse::<u16>().unwrap_or(400);
            (code, parts[1].to_string())
        } else {
            (400, error)
        }
    } else {
        (400, error)
    };

    let response = OutgoingResponse::new(Fields::new());
    response.set_status_code(status_code).ok();

    // Set Content-Type header for JSON error
    let headers = response.headers();
    headers
        .set("content-type", &[b"application/json".to_vec()])
        .ok();

    // Write error message to body
    // Per spec: "The HTTP response body MAY comprise a JSON-RPC error response that has no id"
    if let Ok(body) = response.body() {
        if let Ok(stream) = body.write() {
            let error_json = serde_json::json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32700,
                    "message": message
                },
                "id": null
            });
            let error_text = serde_json::to_string(&error_json).unwrap_or_else(|_| message.clone());

            // Write using check_write() to respect budget
            let bytes = error_text.as_bytes();
            let mut offset = 0;
            while offset < bytes.len() {
                if let Ok(budget) = stream.check_write() {
                    if budget == 0 {
                        break;
                    }
                    let chunk_size = (bytes.len() - offset).min(budget as usize);
                    let chunk = &bytes[offset..offset + chunk_size];
                    if stream.write(chunk).is_err() {
                        break;
                    }
                    offset += chunk_size;
                } else {
                    break;
                }
            }

            drop(stream);
            OutgoingBody::finish(body, None).ok();
        }
    }

    response
}

/// Create 405 Method Not Allowed response
///
/// Used when HTTP method is not POST, GET, or DELETE.
pub fn create_method_not_allowed_response() -> Result<OutgoingResponse, String> {
    let response = OutgoingResponse::new(Fields::new());
    response
        .set_status_code(405)
        .map_err(|_| "Failed to set status")?;
    Ok(response)
}

/// Read entire request body into memory
///
/// Reads body stream in chunks until EOF or error.
/// Properly drops child stream resource before parent body.
///
/// # Arguments
/// * `body` - Incoming request body
///
/// # Returns
/// * `Ok(Vec<u8>)` - Complete body bytes
/// * `Err(String)` - If stream read fails
pub fn read_request_body(body: IncomingBody) -> Result<Vec<u8>, String> {
    let stream = body.stream().map_err(|_| "Failed to get body stream")?;
    let mut buffer = Vec::new();

    loop {
        match stream.blocking_read(8192) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    break;
                }
                buffer.extend_from_slice(&chunk);
            }
            Err(StreamError::Closed) => break,
            Err(e) => {
                drop(stream); // Explicit cleanup before error
                return Err(format!("Stream error: {:?}", e));
            }
        }
    }

    // Explicitly drop stream child resource before parent body is dropped
    drop(stream);

    Ok(buffer)
}

/// Parse request ID from JSON value
///
/// Wrapper around parser module for convenience.
pub fn parse_request_id(id: &serde_json::Value) -> Result<RequestId, String> {
    parser::parse_request_id(id)
}
