//! OAuth 2.0 Discovery Endpoints
//!
//! Implements RFC-compliant discovery endpoints:
//! - RFC 9728: OAuth 2.0 Protected Resource Metadata
//! - RFC 8414: OAuth 2.0 Authorization Server Metadata
//! - OIDC: OpenID Connect Discovery

use crate::bindings::wasi::cli::environment::get_environment;
use crate::bindings::wasi::http::types::{IncomingRequest, ResponseOutparam};
use crate::http::response::ResponseBuilder;
use serde_json::json;

/// Default cache TTL for discovery endpoints in seconds (1 hour)
/// Can be overridden via MCP_DISCOVERY_CACHE_TTL environment variable
const DEFAULT_DISCOVERY_CACHE_TTL: u32 = 3600;

/// Get cache TTL from environment or use default
fn get_discovery_cache_ttl() -> String {
    let ttl = get_environment()
        .iter()
        .find(|(k, _)| k == "MCP_DISCOVERY_CACHE_TTL")
        .and_then(|(_, v)| v.parse::<u32>().ok())
        .unwrap_or(DEFAULT_DISCOVERY_CACHE_TTL);

    format!("public, max-age={}", ttl)
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
    eprintln!("[transport:discovery] Serving protected resource metadata");

    let metadata = build_protected_resource_metadata(request);

    let json_body = serde_json::to_string_pretty(&metadata).unwrap_or_else(|_| "{}".to_string());

    eprintln!("[transport:discovery] Metadata: {}", json_body);

    // Build response
    let response = match ResponseBuilder::new()
        .status(200)
        .header("content-type", b"application/json")
        .header("cache-control", get_discovery_cache_ttl().as_bytes())
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[transport:discovery] Failed to build response: {:?}", e);
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

/// Handle /.well-known/oauth-authorization-server (RFC 8414)
///
/// Returns metadata about the authorization server including:
/// - Issuer identifier
/// - Endpoints (authorize, token, etc.)
/// - Supported features
pub fn handle_authorization_server_metadata(
    _request: &IncomingRequest,
    response_out: ResponseOutparam,
) {
    eprintln!("[transport:discovery] Serving authorization server metadata");

    let metadata = build_authorization_server_metadata();

    let json_body = serde_json::to_string_pretty(&metadata).unwrap_or_else(|_| "{}".to_string());

    eprintln!("[transport:discovery] Metadata: {}", json_body);

    // Build response
    let response = match ResponseBuilder::new()
        .status(200)
        .header("content-type", b"application/json")
        .header("cache-control", get_discovery_cache_ttl().as_bytes())
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[transport:discovery] Failed to build response: {:?}", e);
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

/// Handle /.well-known/openid-configuration (OIDC Discovery)
///
/// Returns OpenID Connect configuration including:
/// - Issuer
/// - JWKS URI
/// - Supported grant types, response types
pub fn handle_openid_configuration(_request: &IncomingRequest, response_out: ResponseOutparam) {
    eprintln!("[transport:discovery] Serving OpenID Connect configuration");

    let config = build_openid_configuration();

    let json_body = serde_json::to_string_pretty(&config).unwrap_or_else(|_| "{}".to_string());

    eprintln!("[transport:discovery] Configuration: {}", json_body);

    // Build response
    let response = match ResponseBuilder::new()
        .status(200)
        .header("content-type", b"application/json")
        .header("cache-control", get_discovery_cache_ttl().as_bytes())
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[transport:discovery] Failed to build response: {:?}", e);
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
    eprintln!("[transport:discovery] Resource (server URI): {}", resource);

    // Get authorization server(s) from config
    let auth_servers: Vec<String> = env_vars
        .iter()
        .find(|(k, _)| k == "MCP_AUTH_SERVER_URL")
        .map(|(_, v)| vec![v.clone()])
        .or_else(|| {
            // Try JWT_ISSUER as fallback
            env_vars
                .iter()
                .find(|(k, _)| k == "JWT_ISSUER")
                .map(|(_, v)| vec![v.clone()])
        })
        .unwrap_or_default();

    // Get JWKS URI if configured
    let jwks_uri = env_vars
        .iter()
        .find(|(k, _)| k == "JWT_JWKS_URI")
        .map(|(_, v)| v.clone());

    // Get supported scopes
    let scopes_supported: Vec<String> = env_vars
        .iter()
        .find(|(k, _)| k == "JWT_REQUIRED_SCOPES")
        .map(|(_, v)| v.split(',').map(|s| s.trim().to_string()).collect())
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

/// Build authorization server metadata (RFC 8414)
fn build_authorization_server_metadata() -> serde_json::Value {
    let env_vars = get_environment();

    // Get issuer from config
    let issuer = env_vars
        .iter()
        .find(|(k, _)| k == "JWT_ISSUER")
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| "unknown".to_string());

    // Get JWKS URI
    let jwks_uri = env_vars
        .iter()
        .find(|(k, _)| k == "JWT_JWKS_URI")
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| format!("{}/.well-known/jwks.json", issuer));

    json!({
        "issuer": issuer.clone(),
        "authorization_endpoint": format!("{}/oauth2/authorize", issuer),
        "token_endpoint": format!("{}/oauth2/token", issuer),
        "jwks_uri": jwks_uri,
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "token_endpoint_auth_methods_supported": ["none", "client_secret_basic", "client_secret_post"],
        "code_challenge_methods_supported": ["S256"],
        "scopes_supported": ["openid", "profile", "email", "offline_access"],
    })
}

/// Build OpenID Connect configuration
fn build_openid_configuration() -> serde_json::Value {
    let env_vars = get_environment();

    // Get issuer from config
    let issuer = env_vars
        .iter()
        .find(|(k, _)| k == "JWT_ISSUER")
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| "unknown".to_string());

    // Get JWKS URI
    let jwks_uri = env_vars
        .iter()
        .find(|(k, _)| k == "JWT_JWKS_URI")
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| format!("{}/.well-known/jwks.json", issuer));

    json!({
        "issuer": issuer.clone(),
        "authorization_endpoint": format!("{}/oauth2/authorize", issuer),
        "token_endpoint": format!("{}/oauth2/token", issuer),
        "userinfo_endpoint": format!("{}/oauth2/userinfo", issuer),
        "jwks_uri": jwks_uri,
        "registration_endpoint": format!("{}/oauth2/register", issuer),
        "response_types_supported": ["code", "code id_token"],
        "response_modes_supported": ["query"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["RS256"],
        "token_endpoint_auth_methods_supported": ["none", "client_secret_basic", "client_secret_post"],
        "scopes_supported": ["openid", "profile", "email", "offline_access"],
        "code_challenge_methods_supported": ["S256"],
    })
}

/// Extract server URI from environment or request
fn get_server_uri(env_vars: &[(String, String)], request: &IncomingRequest) -> String {
    // First try environment variable
    if let Some((_, uri)) = env_vars.iter().find(|(k, _)| k == "MCP_SERVER_URI") {
        eprintln!(
            "[transport:discovery] Using MCP_SERVER_URI from env: {}",
            uri
        );
        return uri.clone();
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
        let uri = format!("{}://{}", scheme, host);
        eprintln!(
            "[transport:discovery] Constructed URI from Host header: {} (scheme from request: {:?})",
            uri,
            request.scheme()
        );
        return uri;
    }

    // Last resort fallback
    eprintln!("[transport:discovery] Using fallback URI (no env var, no Host header)");
    "https://localhost:3000".to_string()
}
