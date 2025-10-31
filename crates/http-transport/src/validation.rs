//! Header validation and session ID extraction for HTTP transport
//!
//! Implements MCP spec requirements for:
//! - Session ID format validation (visible ASCII only)
//! - Accept header validation (must include application/json and text/event-stream)
//! - Protocol version validation (2025-06-18, 2025-03-26, 2024-11-05)
//! - Origin header validation (DNS rebinding protection)

use crate::bindings::wasi::cli::environment::get_environment;
use crate::bindings::wasi::http::types::{Fields, IncomingRequest};
use crate::session::validate_session_id_format;

/// Extracts session ID from Mcp-Session-Id header
///
/// Per MCP spec:
/// - Session IDs MUST only contain visible ASCII (0x21-0x7E)
/// - Returns None if header not present
/// - Returns error if header present but invalid format
pub fn extract_session_id(request: &IncomingRequest) -> Result<Option<String>, String> {
    let headers = request.headers();
    let session_id = get_header_value(&headers, "Mcp-Session-Id")?;

    let Some(session_id) = session_id else {
        return Ok(None);
    };

    // Validate format per MCP spec
    validate_session_id_format(&session_id)
        .map_err(|e| format!("invalid session ID format: {:?}", e))?;

    Ok(Some(session_id))
}

/// Validates Accept header per MCP spec
///
/// Per MCP spec: "The client MUST include an Accept header, listing both application/json
/// and text/event-stream as supported content types"
/// https://modelcontextprotocol.io/specification/2025-06-18/basic/transports#sending-messages-to-the-server
pub fn validate_accept_header(request: &IncomingRequest) -> Result<(), String> {
    let headers = request.headers();
    let accept_str = get_header_value(&headers, "accept")?.ok_or("missing Accept header")?;

    // Check if both required content types are present
    let has_json = accept_str.contains("application/json") || accept_str.contains("*/*");
    let has_sse = accept_str.contains("text/event-stream") || accept_str.contains("*/*");

    if !has_json || !has_sse {
        return Err(
            "accept header must include both application/json and text/event-stream".to_string(),
        );
    }

    Ok(())
}

/// Validates MCP-Protocol-Version header
///
/// Per MCP spec: If using HTTP, the client MUST include the MCP-Protocol-Version header
/// https://modelcontextprotocol.io/specification/2025-06-18/basic/transports#protocol-version-header
///
/// # Returns
/// * `Ok(String)` - Validated protocol version
/// * `Err(String)` - If version is unsupported
pub fn validate_protocol_version(request: &IncomingRequest) -> Result<String, String> {
    let headers = request.headers();
    let version_str = get_header_value(&headers, "mcp-protocol-version")?;

    let version_str = match version_str {
        Some(v) => v,
        None => {
            // No version header - assume 2025-03-26 for backwards compatibility
            // Per spec: "the server SHOULD assume protocol version 2025-03-26"
            return Ok("2025-03-26".to_string());
        }
    };

    // Validate supported versions
    match version_str.as_str() {
        "2025-06-18" | "2025-03-26" | "2024-11-05" => Ok(version_str),
        _ => Err(format!("unsupported MCP-Protocol-Version: {}", version_str)),
    }
}

/// Validate Origin header to prevent DNS rebinding attacks
///
/// Per MCP spec: Servers MUST validate the Origin header to prevent DNS rebinding attacks
/// https://modelcontextprotocol.io/specification/2025-06-18/basic/transports#security-warning
///
/// Configuration via environment variables:
/// - MCP_ALLOWED_ORIGINS: Comma-separated list of allowed origins (e.g., "http://localhost:3000,https://app.example.com")
///   - Special value "*" allows all origins (INSECURE - only for development)
/// - MCP_REQUIRE_ORIGIN: "true" to require Origin header, "false" to allow missing Origin (default: false)
///
/// Default behavior: If MCP_ALLOWED_ORIGINS is not set, only localhost origins are allowed.
pub fn validate_origin(request: &IncomingRequest) -> Result<(), String> {
    let origin = extract_origin_header(request)?;

    // If no Origin header and not required, allow (non-browser clients)
    let Some(origin_value) = origin else {
        return Ok(());
    };

    let env_vars = get_environment();
    let allowed_origins = get_env_var(&env_vars, "MCP_ALLOWED_ORIGINS");

    match allowed_origins {
        Some(allowed) => check_allowed_origins(&origin_value, allowed),
        None => validate_localhost_origin(&origin_value),
    }
}

/// Extract and validate Origin header from request
fn extract_origin_header(request: &IncomingRequest) -> Result<Option<String>, String> {
    let headers = request.headers();
    let origin = get_header_value(&headers, "origin")?;

    if origin.is_none() {
        let env_vars = get_environment();
        let require_origin = get_env_var(&env_vars, "MCP_REQUIRE_ORIGIN").unwrap_or("false");

        if require_origin == "true" {
            return Err("origin header required but not provided".to_string());
        }
    }

    Ok(origin)
}

/// Check if origin is in allowed origins list
fn check_allowed_origins(origin: &str, allowed: &str) -> Result<(), String> {
    let allowed_list: Vec<&str> = allowed.split(',').map(|s| s.trim()).collect();

    // Special case: "*" means allow all (INSECURE - development only)
    if allowed_list.contains(&"*") {
        return Ok(());
    }

    // Check if origin is in allowed list
    if allowed_list.contains(&origin) {
        Ok(())
    } else {
        Err(format!(
            "origin '{}' not in allowed list; set MCP_ALLOWED_ORIGINS environment variable",
            origin
        ))
    }
}

/// Helper to get environment variable value
fn get_env_var<'a>(env_vars: &'a [(String, String)], key: &str) -> Option<&'a str> {
    env_vars
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

/// Helper to extract and decode header value
///
/// Returns None if header is missing, or error if decoding fails.
fn get_header_value(headers: &Fields, name: &str) -> Result<Option<String>, String> {
    let values = headers.get(name);

    if values.is_empty() {
        return Ok(None);
    }

    let value = String::from_utf8(values[0].clone())
        .map_err(|_| format!("invalid {} header encoding", name))?;

    Ok(Some(value))
}

/// Validate that origin is a localhost origin (default secure behavior)
fn validate_localhost_origin(origin: &str) -> Result<(), String> {
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

    Err(format!(
        "origin '{}' not allowed; by default, only localhost origins are permitted; \
        set MCP_ALLOWED_ORIGINS environment variable to allow other origins",
        origin
    ))
}
