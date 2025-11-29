//! OAuth 2.0 Discovery Endpoint
//!
//! Implements RFC 9728: OAuth 2.0 Protected Resource Metadata
//!
//! This endpoint tells clients which authorization servers protect this MCP server.

use crate::bindings::wasi::cli::environment::get_environment;
use crate::bindings::wasi::http::types::{IncomingRequest, ResponseOutparam};
use crate::http::helpers::{get_env, get_server_uri};
use crate::http::response::ResponseBuilder;
use serde_json::json;

/// Default cache TTL for discovery endpoints in seconds (1 hour)
/// Can be overridden via WASMCP_DISCOVERY_CACHE_TTL environment variable
const DEFAULT_DISCOVERY_CACHE_TTL: u32 = 3600;

/// Get cache TTL from environment or use default
fn get_discovery_cache_ttl() -> String {
    let ttl = get_environment()
        .iter()
        .find(|(k, _)| k == "WASMCP_DISCOVERY_CACHE_TTL")
        .and_then(|(_, v)| v.parse::<u32>().ok())
        .unwrap_or(DEFAULT_DISCOVERY_CACHE_TTL);

    format!("public, max-age={}", ttl)
}

/// Parse comma-separated values
fn parse_comma_separated(value: &str) -> Vec<String> {
    value.split(',').map(|s| s.trim().to_string()).collect()
}

/// Handle /.well-known/oauth-protected-resource (RFC 9728)
///
/// Returns metadata about this protected resource server including:
/// - Resource identifier (canonical URI)
/// - Authorization servers that protect this resource
/// - Supported scopes
/// - Bearer token methods
pub fn handle_protected_resource_metadata(
    request: &IncomingRequest,
    response_out: ResponseOutparam,
) {
    let metadata = build_protected_resource_metadata(request);

    let json_body = serde_json::to_string_pretty(&metadata).unwrap_or_else(|_| "{}".to_string());

    // Build response
    let response = match ResponseBuilder::new()
        .status(200)
        .header("content-type", b"application/json")
        .header("cache-control", get_discovery_cache_ttl().as_bytes())
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            let error_response = crate::http::response::transport_error_to_response(&e);
            ResponseOutparam::set(response_out, Ok(error_response));
            return;
        }
    };

    // Write JSON body
    if let Ok(body) = response.body() {
        if let Ok(stream) = body.write() {
            let _ = stream.blocking_write_and_flush(json_body.as_bytes());
            drop(stream);
        }
        let _ = crate::bindings::wasi::http::types::OutgoingBody::finish(body, None);
    }

    ResponseOutparam::set(response_out, Ok(response));
}

/// Build protected resource metadata (RFC 9728)
fn build_protected_resource_metadata(request: &IncomingRequest) -> serde_json::Value {
    let env_vars = get_environment();

    // Get resource identifier - ALWAYS use actual server URI
    // MCP clients validate that resource field matches connection URL
    let resource = get_server_uri(&env_vars, request);

    // Get authorization server(s) from config
    let auth_servers: Vec<String> = get_env(&env_vars, "WASMCP_AUTH_SERVER_URL")
        .or_else(|| get_env(&env_vars, "JWT_ISSUER"))
        .map(|v| vec![v])
        .unwrap_or_default();

    // Get JWKS URI if configured
    let jwks_uri = get_env(&env_vars, "JWT_JWKS_URI");

    // Get supported scopes
    let scopes_supported: Vec<String> = get_env(&env_vars, "JWT_REQUIRED_SCOPES")
        .map(|v| parse_comma_separated(&v))
        .unwrap_or_default();

    json!({
        "resource": resource,
        "authorization_servers": auth_servers,
        "jwks_uri": jwks_uri,
        "scopes_supported": scopes_supported,
        "bearer_methods_supported": ["header"],
        "resource_documentation": format!("{}/.well-known/oauth-protected-resource", resource),
    })
}
