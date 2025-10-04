//! WASI HTTP client implementation for OTLP export.

use crate::auth;
use crate::bindings::wasi::http::outgoing_handler;
use crate::bindings::wasi::http::types::{
    Fields, Method, OutgoingBody, OutgoingRequest, Scheme,
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

    // Send the request
    let incoming_response = match outgoing_handler::handle(request, None) {
        Ok(resp) => resp,
        Err(_) => return ExportResult::Failure("Failed to send request".to_string()),
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
        200..=299 => ExportResult::Success,
        400..=499 => {
            // Client error - don't retry
            ExportResult::Failure(format!("Client error: HTTP {}", status))
        }
        500..=599 => {
            // Server error - retry
            ExportResult::Failure(format!("Server error: HTTP {}", status))
        }
        _ => ExportResult::Failure(format!("Unexpected status: HTTP {}", status)),
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