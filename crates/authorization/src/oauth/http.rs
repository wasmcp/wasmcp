//! Shared HTTP GET utility for OAuth discovery endpoints

use crate::bindings::wasi::http::outgoing_handler;
use crate::bindings::wasi::http::types::{Fields, Method, OutgoingBody, OutgoingRequest, Scheme};
use crate::bindings::wasi::io::poll;
use crate::bindings::wasi::io::streams::StreamError;

/// Perform a blocking HTTP GET and return the response body as a String.
///
/// Returns `Err(message)` on any network, HTTP, or I/O failure.
pub fn http_get(url: &str) -> Result<String, String> {
    let parsed = url
        .parse::<url::Url>()
        .map_err(|e| format!("Invalid URL '{}': {}", url, e))?;

    let scheme = match parsed.scheme() {
        "https" => Scheme::Https,
        "http" => Scheme::Http,
        s => return Err(format!("Unsupported URL scheme: {}", s)),
    };

    let authority = parsed
        .host_str()
        .ok_or_else(|| format!("No host in URL: {}", url))?
        .to_string();
    let authority = if let Some(port) = parsed.port() {
        format!("{}:{}", authority, port)
    } else {
        authority
    };

    let path_and_query = match parsed.query() {
        Some(q) => format!("{}?{}", parsed.path(), q),
        None => parsed.path().to_string(),
    };

    let headers = Fields::new();
    headers
        .append("Accept", b"application/json")
        .map_err(|_| "Failed to set Accept header".to_string())?;

    let request = OutgoingRequest::new(headers);
    request
        .set_method(&Method::Get)
        .map_err(|_| "Failed to set GET method".to_string())?;
    request
        .set_scheme(Some(&scheme))
        .map_err(|_| "Failed to set scheme".to_string())?;
    request
        .set_authority(Some(&authority))
        .map_err(|_| "Failed to set authority".to_string())?;
    request
        .set_path_with_query(Some(&path_and_query))
        .map_err(|_| "Failed to set path".to_string())?;

    // Finish the (empty) body
    let outgoing_body = request
        .body()
        .map_err(|_| "Failed to get request body".to_string())?;
    OutgoingBody::finish(outgoing_body, None)
        .map_err(|_| "Failed to finish request body".to_string())?;

    let future_response =
        outgoing_handler::handle(request, None).map_err(|e| format!("Request failed: {:?}", e))?;

    let pollable = future_response.subscribe();
    poll::poll(&[&pollable]);
    drop(pollable);

    let response = future_response
        .get()
        .ok_or("Response not ready")?
        .map_err(|e| format!("Future error: {:?}", e))?
        .map_err(|e| format!("HTTP error: {:?}", e))?;

    let status = response.status();

    let body = response
        .consume()
        .map_err(|_| "Failed to get response body".to_string())?;
    let stream = body
        .stream()
        .map_err(|_| "Failed to get response stream".to_string())?;

    let mut bytes = Vec::new();
    loop {
        match stream.blocking_read(4096) {
            Ok(chunk) if chunk.is_empty() => break,
            Ok(chunk) => bytes.extend_from_slice(&chunk),
            Err(StreamError::Closed) => break,
            Err(e) => return Err(format!("Failed to read response body: {:?}", e)),
        }
    }

    let body_str =
        String::from_utf8(bytes).map_err(|e| format!("Invalid UTF-8 in response: {}", e))?;

    if status != 200 {
        return Err(format!("HTTP {} from {}: {}", status, url, body_str));
    }

    Ok(body_str)
}
