//! Simple HTTP client using WASI HTTP interfaces

use crate::wit::wasi::http0_2_0::outgoing_handler;
use crate::wit::wasi::http0_2_0::types;
use anyhow::Result;

/// Simple HTTP request builder
pub struct Request {
    method: types::Method,
    url: String,
    headers: Vec<(String, Vec<u8>)>,
    body: Option<Vec<u8>>,
}

impl Request {
    /// Create a GET request
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            method: types::Method::Get,
            url: url.into(),
            headers: Vec::new(),
            body: None,
        }
    }
    
    /// Create a POST request
    pub fn post(url: impl Into<String>) -> Self {
        Self {
            method: types::Method::Post,
            url: url.into(),
            headers: Vec::new(),
            body: None,
        }
    }
    
    /// Add a header
    pub fn header(mut self, name: impl Into<String>, value: impl Into<Vec<u8>>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
    
    /// Set the body
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = Some(body.into());
        self
    }
}

/// Simple HTTP response
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, Vec<u8>)>,
    pub body: Vec<u8>,
}

/// Send an HTTP request
pub async fn send(request: Request) -> Result<Response> {
    // Parse URL to extract components
    let url = url::Url::parse(&request.url)?;
    
    // Create headers
    let headers = types::Headers::from_list(
        &request.headers
            .into_iter()
            .map(|(k, v)| (k, v))
            .collect::<Vec<_>>()
    )?;
    
    // Create outgoing request
    let outgoing_request = types::OutgoingRequest::new(headers);
    
    // Set method
    outgoing_request.set_method(&request.method)
        .map_err(|_| anyhow::anyhow!("Failed to set method"))?;
    
    // Set path with query
    let path_with_query = if let Some(query) = url.query() {
        format!("{}?{}", url.path(), query)
    } else {
        url.path().to_string()
    };
    outgoing_request.set_path_with_query(Some(&path_with_query))
        .map_err(|_| anyhow::anyhow!("Failed to set path with query"))?;
    
    // Set scheme
    let scheme = match url.scheme() {
        "http" => types::Scheme::Http,
        "https" => types::Scheme::Https,
        other => types::Scheme::Other(other.to_string()),
    };
    outgoing_request.set_scheme(Some(&scheme))
        .map_err(|_| anyhow::anyhow!("Failed to set scheme"))?;
    
    // Set authority (host:port)
    if let Some(host) = url.host_str() {
        let authority = if let Some(port) = url.port() {
            format!("{}:{}", host, port)
        } else {
            host.to_string()
        };
        outgoing_request.set_authority(Some(&authority))
            .map_err(|_| anyhow::anyhow!("Failed to set authority"))?;
    }
    
    // Write body if present
    if let Some(body_data) = request.body {
        let body = outgoing_request.body()
            .map_err(|_| anyhow::anyhow!("Failed to get body"))?;
        let stream = body.write()
            .map_err(|_| anyhow::anyhow!("Failed to get write stream"))?;
        stream.blocking_write_and_flush(&body_data)?;
        drop(stream);
        types::OutgoingBody::finish(body, None)?;
    }
    
    // Send request
    let future_response = outgoing_handler::handle(outgoing_request, None)
        .map_err(|e| anyhow::anyhow!("Failed to send request: {:?}", e))?;
    
    // Wait for response
    let incoming_response = loop {
        if let Some(result) = future_response.get() {
            // result is Some(Result<IncomingResponse, ErrorCode>)
            // We need to handle both the Option and the Result
            break result.unwrap()
                .map_err(|e| anyhow::anyhow!("HTTP request failed: {:?}", e))?;
        }
        future_response.subscribe().block();
    };
    
    // Get response status
    let status = incoming_response.status();
    
    // Get response headers  
    let response_headers = incoming_response.headers();
    let headers = response_headers.entries();
    
    // Read response body
    let incoming_body = incoming_response.consume()
        .map_err(|_| anyhow::anyhow!("Failed to consume response body"))?;
    let stream = incoming_body.stream()
        .map_err(|_| anyhow::anyhow!("Failed to get body stream"))?;
    
    let mut body = Vec::new();
    loop {
        let chunk = stream.read(64 * 1024)?; // Read up to 64KB at a time
        if chunk.is_empty() {
            break;
        }
        body.extend_from_slice(&chunk);
    }
    
    Ok(Response {
        status,
        headers,
        body,
    })
}

// Re-export url for convenience
pub use url;