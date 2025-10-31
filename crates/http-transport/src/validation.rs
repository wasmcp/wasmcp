//! Header validation and session ID extraction for HTTP transport
//!
//! Implements MCP spec requirements for:
//! - Session ID format validation (visible ASCII only)
//! - Accept header validation (must include application/json and text/event-stream)
//! - Protocol version validation (2025-06-18, 2025-03-26, 2024-11-05)
//! - Origin header validation (DNS rebinding protection)

use crate::bindings::wasi::cli::environment::get_environment;
use crate::bindings::wasi::http::types::IncomingRequest;
use crate::session::validate_session_id_format;

/// Extracts session ID from Mcp-Session-Id header
///
/// Per MCP spec:
/// - Session IDs MUST only contain visible ASCII (0x21-0x7E)
/// - Returns None if header not present
/// - Returns error if header present but invalid format
pub fn extract_session_id(request: &IncomingRequest) -> Result<Option<String>, String> {
    let headers = request.headers();
    let session_values = headers.get(&"Mcp-Session-Id".to_string());

    if session_values.is_empty() {
        return Ok(None);
    }

    let session_id = String::from_utf8(session_values[0].clone())
        .map_err(|_| "Invalid Mcp-Session-Id header encoding".to_string())?;

    // Validate format per MCP spec
    validate_session_id_format(&session_id)
        .map_err(|e| format!("Invalid session ID format: {:?}", e))?;

    Ok(Some(session_id))
}

/// Validates Accept header per MCP spec
///
/// Per MCP spec: "The client MUST include an Accept header, listing both application/json
/// and text/event-stream as supported content types"
/// https://modelcontextprotocol.io/specification/2025-06-18/basic/transports#sending-messages-to-the-server
pub fn validate_accept_header(request: &IncomingRequest) -> Result<(), String> {
    let headers = request.headers();
    let accept_values = headers.get("accept");

    if accept_values.is_empty() {
        return Err("Missing Accept header".to_string());
    }

    let accept_str = String::from_utf8(accept_values[0].clone())
        .map_err(|_| "Invalid Accept header encoding".to_string())?;

    // Check if both required content types are present
    let has_json = accept_str.contains("application/json") || accept_str.contains("*/*");
    let has_sse = accept_str.contains("text/event-stream") || accept_str.contains("*/*");

    if !has_json || !has_sse {
        return Err(
            "Accept header must include both application/json and text/event-stream".to_string(),
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
    let version_values = headers.get("mcp-protocol-version");

    if version_values.is_empty() {
        // No version header - assume 2025-03-26 for backwards compatibility
        // Per spec: "the server SHOULD assume protocol version 2025-03-26"
        return Ok("2025-03-26".to_string());
    }

    let version_str = String::from_utf8(version_values[0].clone())
        .map_err(|_| "Invalid MCP-Protocol-Version header encoding".to_string())?;

    // Validate supported versions
    match version_str.as_str() {
        "2025-06-18" | "2025-03-26" | "2024-11-05" => Ok(version_str),
        _ => Err(format!("Unsupported MCP-Protocol-Version: {}", version_str)),
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
    // Get the Origin header
    let headers = request.headers();
    let origin_values = headers.get("origin");

    // Get environment variables
    let env_vars = get_environment();
    let require_origin = env_vars
        .iter()
        .find(|(k, _)| k == "MCP_REQUIRE_ORIGIN")
        .map(|(_, v)| v.as_str())
        .unwrap_or("false");

    let allowed_origins = env_vars
        .iter()
        .find(|(k, _)| k == "MCP_ALLOWED_ORIGINS")
        .map(|(_, v)| v.as_str());

    // If no Origin header, check if we require it
    let origin = if origin_values.is_empty() {
        if require_origin == "true" {
            return Err("Origin header required but not provided".to_string());
        }
        // No Origin header but not required - allow (non-browser clients)
        return Ok(());
    } else {
        // Take first Origin value and decode
        String::from_utf8(origin_values[0].clone())
            .map_err(|_| "Invalid Origin header encoding".to_string())?
    };

    // Check allowed origins
    match allowed_origins {
        Some(allowed) => {
            // Comma-separated list of allowed origins
            let allowed_list: Vec<&str> = allowed.split(',').map(|s| s.trim()).collect();

            // Special case: "*" means allow all (INSECURE - development only)
            if allowed_list.contains(&"*") {
                return Ok(());
            }

            // Check if origin is in allowed list
            if allowed_list.contains(&origin.as_str()) {
                Ok(())
            } else {
                Err(format!(
                    "Origin '{}' not in allowed list. Set MCP_ALLOWED_ORIGINS environment variable.",
                    origin
                ))
            }
        }
        None => {
            // No configuration - default to localhost only for security
            validate_localhost_origin(&origin)
        }
    }
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
        "Origin '{}' not allowed. By default, only localhost origins are permitted. \
        Set MCP_ALLOWED_ORIGINS environment variable to allow other origins.",
        origin
    ))
}
