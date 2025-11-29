//! HTTP header validation functions
//!
//! This module contains all header extraction and validation logic for the HTTP transport.
//! Functions here validate:
//! - Session IDs (Mcp-Session-Id header)
//! - JWT tokens (Authorization header)
//! - Accept headers (application/json and text/event-stream)
//! - Protocol versions (MCP-Protocol-Version header)
//! - Origins (Origin header for DNS rebinding protection)

use crate::bindings::wasi::cli::environment::get_environment;
use crate::bindings::wasi::http::types::IncomingRequest;
use crate::error::TransportError;

/// Supported MCP protocol versions
///
/// These versions are accepted in the MCP-Protocol-Version header.
/// Newest versions should be added to the front of the array.
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &[
    "2025-06-18",
    "2025-03-26",
    "2024-11-05",
];

/// Default protocol version for backwards compatibility
const DEFAULT_PROTOCOL_VERSION: &str = "2025-03-26";

/// Extract session ID from request headers
pub fn extract_session_id_header(
    request: &IncomingRequest,
) -> Result<Option<String>, TransportError> {
    let headers = request.headers();
    let session_id_values = headers.get("mcp-session-id");

    if session_id_values.is_empty() {
        return Ok(None);
    }

    String::from_utf8(session_id_values[0].clone())
        .map(Some)
        .map_err(|_| TransportError::validation("Invalid Mcp-Session-Id header encoding"))
}

/// Extract JWT bearer token from Authorization header
pub fn extract_authorization_header(
    request: &IncomingRequest,
) -> Result<Option<Vec<u8>>, TransportError> {
    let headers = request.headers();
    let auth_values = headers.get("authorization");

    if auth_values.is_empty() {
        return Ok(None);
    }

    let auth_str = String::from_utf8(auth_values[0].clone())
        .map_err(|_| TransportError::validation("Invalid Authorization header encoding"))?;

    if let Some(token) = auth_str.strip_prefix("Bearer ") {
        Ok(Some(token.as_bytes().to_vec()))
    } else {
        Err(TransportError::validation(
            "Authorization header must use Bearer scheme",
        ))
    }
}

/// Validate Accept header per MCP spec
pub fn validate_accept_header(request: &IncomingRequest) -> Result<(), TransportError> {
    let headers = request.headers();
    let accept_values = headers.get("accept");

    if accept_values.is_empty() {
        return Err(TransportError::validation("Missing Accept header"));
    }

    let accept_str = String::from_utf8(accept_values[0].clone())
        .map_err(|_| TransportError::validation("Invalid Accept header encoding"))?;

    let has_json = accept_str.contains("application/json") || accept_str.contains("*/*");
    let has_sse = accept_str.contains("text/event-stream") || accept_str.contains("*/*");

    if !has_json || !has_sse {
        return Err(TransportError::validation(
            "Accept header must include both application/json and text/event-stream",
        ));
    }

    Ok(())
}

/// Validate MCP-Protocol-Version header
pub fn validate_protocol_version(request: &IncomingRequest) -> Result<String, TransportError> {
    let headers = request.headers();
    let version_values = headers.get("mcp-protocol-version");

    if version_values.is_empty() {
        // Default for backwards compatibility
        return Ok(DEFAULT_PROTOCOL_VERSION.to_string());
    }

    let version_str = String::from_utf8(version_values[0].clone())
        .map_err(|_| TransportError::validation("Invalid MCP-Protocol-Version header encoding"))?;

    if SUPPORTED_PROTOCOL_VERSIONS.contains(&version_str.as_str()) {
        Ok(version_str)
    } else {
        Err(TransportError::protocol(format!(
            "Unsupported MCP-Protocol-Version: {}. Supported versions: {}",
            version_str,
            SUPPORTED_PROTOCOL_VERSIONS.join(", ")
        )))
    }
}

/// Validate Origin header to prevent DNS rebinding attacks
pub fn validate_origin(request: &IncomingRequest) -> Result<(), TransportError> {
    let headers = request.headers();
    let origin_values = headers.get("origin");

    let env_vars = get_environment();
    let require_origin = env_vars
        .iter()
        .find(|(k, _)| k == "WASMCP_REQUIRE_ORIGIN")
        .map(|(_, v)| v.as_str())
        .unwrap_or("false");

    let allowed_origins = env_vars
        .iter()
        .find(|(k, _)| k == "WASMCP_ALLOWED_ORIGINS")
        .map(|(_, v)| v.as_str());

    let origin = if origin_values.is_empty() {
        if require_origin == "true" {
            return Err(TransportError::validation(
                "Origin header required but not provided",
            ));
        }
        return Ok(());
    } else {
        String::from_utf8(origin_values[0].clone())
            .map_err(|_| TransportError::validation("Invalid Origin header encoding"))?
    };

    match allowed_origins {
        Some(allowed) => {
            let allowed_list: Vec<&str> = allowed.split(',').map(|s| s.trim()).collect();

            if allowed_list.contains(&"*") {
                return Ok(());
            }

            if allowed_list.contains(&origin.as_str()) {
                Ok(())
            } else {
                Err(TransportError::validation(format!(
                    "Origin '{}' not in allowed list. Set WASMCP_ALLOWED_ORIGINS environment variable.",
                    origin
                )))
            }
        }
        None => validate_localhost_origin(&origin),
    }
}

/// Validate localhost origin (default secure behavior)
pub fn validate_localhost_origin(origin: &str) -> Result<(), TransportError> {
    let localhost_patterns = [
        "http://localhost",
        "https://localhost",
        "http://127.0.0.1",
        "https://127.0.0.1",
        "http://[::1]",
        "https://[::1]",
    ];

    for pattern in &localhost_patterns {
        if origin.starts_with(pattern) {
            return Ok(());
        }
    }

    Err(TransportError::validation(format!(
        "Origin '{}' not allowed. By default, only localhost origins are permitted.",
        origin
    )))
}
