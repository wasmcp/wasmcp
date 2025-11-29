//! HTTP helper utilities
//!
//! Shared helper functions for HTTP request handling

use crate::bindings::wasi::http::types::IncomingRequest;

/// Get environment variable value by key
///
/// Searches through environment variable list for matching key.
pub fn get_env(env_vars: &[(String, String)], key: &str) -> Option<String> {
    env_vars
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.clone())
}

/// Extract server URI from environment or request
///
/// Tries in order:
/// 1. WASMCP_SERVER_URI environment variable
/// 2. Construct from Host header + scheme
/// 3. Fallback to https://localhost:3000
///
/// Returns the canonical server URI for this MCP server.
pub fn get_server_uri(env_vars: &[(String, String)], request: &IncomingRequest) -> String {
    // First try environment variable
    if let Some(uri) = get_env(env_vars, "WASMCP_SERVER_URI") {
        return uri;
    }

    // Fall back to constructing from Host header
    let headers = request.headers();
    let host_values = headers.get("host");

    if !host_values.is_empty()
        && let Ok(host) = String::from_utf8(host_values[0].clone())
    {
        // Use scheme from request, default to https if not available
        let scheme = request
            .scheme()
            .and_then(|s| match s {
                crate::bindings::wasi::http::types::Scheme::Http => Some("http"),
                crate::bindings::wasi::http::types::Scheme::Https => Some("https"),
                _ => None,
            })
            .unwrap_or("https");
        return format!("{}://{}", scheme, host);
    }

    // Last resort fallback
    "https://localhost:3000".to_string()
}
