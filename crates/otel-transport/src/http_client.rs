//! WASI HTTP client implementation for OTLP export.

use crate::auth;
use crate::bindings::wasi::http::outgoing_handler;
use crate::bindings::wasi::http::types::{
    Fields, IncomingResponse, Method, OutgoingBody, OutgoingRequest, RequestOptions, Scheme,
};
use crate::bindings::exports::wasi::otel_sdk::otel_export::{
    CompressionType, ExportConfig, ExportResult, RetryConfig,
};

use std::time::Duration;
use std::io::Write;
use flate2::write::GzEncoder;
use flate2::Compression;

/// Send an OTLP request to the specified endpoint
pub fn send_otlp_request(
    config: &ExportConfig,
    signal_path: &str,
    otlp_payload: &[u8],
    content_type: &str,
) -> ExportResult {
    // Build full URL
    let full_url = format!("{}{}", config.endpoint, signal_path);

    // Create the request with retries
    let retry_config = config.retry.as_ref().map(|r| r.clone()).unwrap_or_else(|| RetryConfig {
        max_attempts: 3,
        initial_delay_ms: 1000,
        max_delay_ms: 32000,
    });

    let mut attempts = 0;
    let mut delay_ms = retry_config.initial_delay_ms;

    loop {
        attempts += 1;

        match send_single_request(config, &full_url, otlp_payload, content_type) {
            ExportResult::Success => return ExportResult::Success,
            ExportResult::Failure(err) if attempts >= retry_config.max_attempts => {
                return ExportResult::Failure(format!("Max retries exceeded: {}", err));
            }
            ExportResult::PartialFailure(err) if attempts >= retry_config.max_attempts => {
                return ExportResult::PartialFailure(err);
            }
            result => {
                // Retry with exponential backoff
                if attempts < retry_config.max_attempts {
                    std::thread::sleep(Duration::from_millis(delay_ms as u64));
                    delay_ms = (delay_ms * 2).min(retry_config.max_delay_ms);
                } else {
                    return result;
                }
            }
        }
    }
}

/// Send a single HTTP request (without retries)
fn send_single_request(
    config: &ExportConfig,
    url: &str,
    payload: &[u8],
    content_type: &str,
) -> ExportResult {
    // Parse URL to extract components
    let (scheme, authority, path) = match parse_url(url) {
        Ok(components) => components,
        Err(e) => return ExportResult::Failure(format!("Invalid URL: {}", e)),
    };

    // Apply compression first (before setting content-length)
    let final_payload = match config.compression {
        CompressionType::Gzip => {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            if let Err(_) = encoder.write_all(payload) {
                return ExportResult::Failure("Failed to compress payload".to_string());
            }
            match encoder.finish() {
                Ok(compressed) => compressed,
                Err(_) => return ExportResult::Failure("Failed to finish compression".to_string()),
            }
        }
        CompressionType::None => payload.to_vec(),
    };

    // Create headers
    let headers = Fields::new();

    // Set content-type header
    if let Err(_) = headers.set(&"content-type".to_string(), &[content_type.as_bytes().to_vec()]) {
        return ExportResult::Failure("Failed to set content-type header".to_string());
    }

    // Set content-length header (using final payload size after compression)
    let content_length = final_payload.len().to_string();
    if let Err(_) = headers.set(&"content-length".to_string(), &[content_length.as_bytes().to_vec()]) {
        return ExportResult::Failure("Failed to set content-length header".to_string());
    }

    // Set content-encoding header if compressed
    if matches!(config.compression, CompressionType::Gzip) {
        if let Err(_) = headers.set(&"content-encoding".to_string(), &[b"gzip".to_vec()]) {
            return ExportResult::Failure("Failed to set content-encoding header".to_string());
        }
    }

    // Apply authentication
    if let Err(e) = auth::apply_authentication(&config.authentication, &headers) {
        return ExportResult::Failure(format!("Failed to apply authentication: {}", e));
    }

    // Create the request
    let request = OutgoingRequest::new(headers);

    // Set request properties
    if let Err(_) = request.set_method(&Method::Post) {
        return ExportResult::Failure("Failed to set method".to_string());
    }

    if let Err(_) = request.set_scheme(Some(&scheme)) {
        return ExportResult::Failure("Failed to set scheme".to_string());
    }

    if let Err(_) = request.set_authority(Some(&authority)) {
        return ExportResult::Failure("Failed to set authority".to_string());
    }

    if let Err(_) = request.set_path_with_query(Some(&path)) {
        return ExportResult::Failure("Failed to set path".to_string());
    }

    // Write the body
    let body = match request.body() {
        Ok(b) => b,
        Err(_) => return ExportResult::Failure("Failed to get request body".to_string()),
    };

    let stream = match body.write() {
        Ok(s) => s,
        Err(_) => return ExportResult::Failure("Failed to get body stream".to_string()),
    };

    // Write payload in chunks if necessary
    let chunk_size = 4096;
    for chunk in final_payload.chunks(chunk_size) {
        match stream.blocking_write_and_flush(chunk) {
            Ok(_) => {},
            Err(_) => return ExportResult::Failure("Failed to write request body".to_string()),
        }
    }

    // Finish the body
    drop(stream);
    if let Err(_) = OutgoingBody::finish(body, None) {
        return ExportResult::Failure("Failed to finish request body".to_string());
    }

    // Create request options with timeout
    // Convert timeout from milliseconds to nanoseconds (WASI HTTP uses nanoseconds)
    let request_opts = RequestOptions::new();
    let timeout_nanos = (config.timeout_ms as u64) * 1_000_000;

    // Set connect timeout (how long to wait for connection establishment)
    if let Err(_) = request_opts.set_connect_timeout(Some(timeout_nanos)) {
        return ExportResult::Failure("Failed to set connect timeout".to_string());
    }

    // Set first-byte timeout (how long to wait for first response byte)
    if let Err(_) = request_opts.set_first_byte_timeout(Some(timeout_nanos)) {
        return ExportResult::Failure("Failed to set first-byte timeout".to_string());
    }

    // Send the request with timeout options
    let incoming_response = match outgoing_handler::handle(request, Some(request_opts)) {
        Ok(resp) => resp,
        Err(_) => return ExportResult::Failure("Request failed or timed out".to_string()),
    };

    // Wait for and process the response
    incoming_response.subscribe().block();

    let response = match incoming_response.get() {
        Some(Ok(resp)) => resp,
        Some(Err(_)) => return ExportResult::Failure("Request failed".to_string()),
        None => return ExportResult::Failure("No response received".to_string()),
    };

    // Check response status
    let response = response.expect("HTTP response unwrap");
    let status = response.status();
    match status {
        200..=299 => {
            // Success status - check for partial failure in response body
            let body = read_response_body(&response);
            check_partial_failure(&body)
        }
        400..=499 => {
            // Client error - don't retry
            // Read response body for better error diagnostics
            let body = read_response_body(&response);
            ExportResult::Failure(format!("Client error: HTTP {}: {}", status, body))
        }
        500..=599 => {
            // Server error - retry
            // Read response body for better error diagnostics
            let body = read_response_body(&response);
            ExportResult::Failure(format!("Server error: HTTP {}: {}", status, body))
        }
        _ => {
            let body = read_response_body(&response);
            ExportResult::Failure(format!("Unexpected status: HTTP {}: {}", status, body))
        }
    }
}

/// Check for OTLP partial failure in response body
/// OTLP protocol returns 200 OK even with partial failures
fn check_partial_failure(body: &str) -> ExportResult {
    use crate::bindings::exports::wasi::otel_sdk::otel_export::ExportError;

    // Empty body means complete success
    if body.trim().is_empty() {
        return ExportResult::Success;
    }

    // Try to parse as JSON to check for partialSuccess field
    // OTLP response format:
    // {
    //   "partialSuccess": {
    //     "rejectedSpans": 5,     // or rejectedMetrics/rejectedLogs
    //     "errorMessage": "Some spans had invalid trace_id"
    //   }
    // }

    // Simple JSON parsing for partialSuccess field
    // We're looking for "rejectedSpans", "rejectedMetrics", or "rejectedLogs" with count > 0
    let has_rejected_spans = body.contains("\"rejectedSpans\"")
        && body.contains(":")
        && !body.contains("\"rejectedSpans\":0")
        && !body.contains("\"rejectedSpans\": 0");

    let has_rejected_metrics = body.contains("\"rejectedDataPoints\"")
        && body.contains(":")
        && !body.contains("\"rejectedDataPoints\":0")
        && !body.contains("\"rejectedDataPoints\": 0");

    let has_rejected_logs = body.contains("\"rejectedLogRecords\"")
        && body.contains(":")
        && !body.contains("\"rejectedLogRecords\":0")
        && !body.contains("\"rejectedLogRecords\": 0");

    if has_rejected_spans || has_rejected_metrics || has_rejected_logs {
        // Extract error message if present
        let error_msg = if let Some(start) = body.find("\"errorMessage\"") {
            if let Some(colon) = body[start..].find(':') {
                let msg_start = start + colon + 1;
                if let Some(quote_start) = body[msg_start..].find('"') {
                    let msg_offset = msg_start + quote_start + 1;
                    if let Some(quote_end) = body[msg_offset..].find('"') {
                        body[msg_offset..msg_offset + quote_end].to_string()
                    } else {
                        "Partial failure (details in response)".to_string()
                    }
                } else {
                    "Partial failure (details in response)".to_string()
                }
            } else {
                "Partial failure (details in response)".to_string()
            }
        } else {
            "Partial failure (some data rejected)".to_string()
        };

        ExportResult::PartialFailure(ExportError {
            failed_count: 0, // We don't parse exact count for simplicity
            error_message: error_msg,
        })
    } else {
        ExportResult::Success
    }
}

/// Read response body for error diagnostics
/// Limits read to 10KB to prevent memory exhaustion
fn read_response_body(response: &IncomingResponse) -> String {
    const MAX_BODY_SIZE: usize = 10 * 1024; // 10KB limit

    match response.consume() {
        Ok(body) => {
            // Read body stream with size limit
            let mut buffer = Vec::new();
            let stream = body.stream().expect("Failed to get body stream");

            loop {
                match stream.blocking_read(MAX_BODY_SIZE as u64) {
                    Ok(chunk) => {
                        if chunk.is_empty() {
                            break;
                        }
                        buffer.extend_from_slice(&chunk);

                        // Enforce size limit
                        if buffer.len() >= MAX_BODY_SIZE {
                            buffer.truncate(MAX_BODY_SIZE);
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }

            // Convert to string, replacing invalid UTF-8 with replacement chars
            String::from_utf8_lossy(&buffer).to_string()
        }
        Err(_) => "(failed to read response body)".to_string(),
    }
}

/// Parse URL into scheme, authority, and path components
fn parse_url(url: &str) -> Result<(Scheme, String, String), String> {
    // Basic URL parsing (simplified)
    let url = url.trim();

    let (scheme, rest) = if url.starts_with("https://") {
        (Scheme::Https, &url[8..])
    } else if url.starts_with("http://") {
        (Scheme::Http, &url[7..])
    } else {
        return Err("URL must start with http:// or https://".to_string());
    };

    // Find the path separator
    let (authority, path) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, "/"),
    };

    Ok((scheme, authority.to_string(), path.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url_https() {
        let result = parse_url("https://api.example.com/v1/traces");
        assert!(result.is_ok());
        let (scheme, authority, path) = result.unwrap();
        assert_eq!(authority, "api.example.com");
        assert_eq!(path, "/v1/traces");
    }

    #[test]
    fn test_parse_url_http() {
        let result = parse_url("http://localhost:4318/v1/traces");
        assert!(result.is_ok());
        let (scheme, authority, path) = result.unwrap();
        assert_eq!(authority, "localhost:4318");
        assert_eq!(path, "/v1/traces");
    }

    #[test]
    fn test_parse_url_no_path() {
        let result = parse_url("https://example.com");
        assert!(result.is_ok());
        let (scheme, authority, path) = result.unwrap();
        assert_eq!(authority, "example.com");
        assert_eq!(path, "/");
    }

    #[test]
    fn test_parse_url_with_port() {
        let result = parse_url("https://example.com:443/v1/traces");
        assert!(result.is_ok());
        let (scheme, authority, path) = result.unwrap();
        assert_eq!(authority, "example.com:443");
        assert_eq!(path, "/v1/traces");
    }

    #[test]
    fn test_parse_url_invalid_scheme() {
        let result = parse_url("ftp://example.com/v1/traces");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must start with http"));
    }

    #[test]
    fn test_parse_url_with_query() {
        let result = parse_url("https://example.com/v1/traces?key=value");
        assert!(result.is_ok());
        let (scheme, authority, path) = result.unwrap();
        assert_eq!(authority, "example.com");
        assert_eq!(path, "/v1/traces?key=value");
    }

    #[test]
    fn test_parse_url_trimming() {
        let result = parse_url("  https://example.com/path  ");
        assert!(result.is_ok());
        let (scheme, authority, path) = result.unwrap();
        assert_eq!(authority, "example.com");
        assert_eq!(path, "/path");
    }
}