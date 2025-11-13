//! OAuth Authentication Component
//!
//! Exports the server-auth interface for OAuth 2.1 Resource Server authentication and authorization.
//! Supports JWT validation, token introspection, and policy-based authorization.

mod bindings {
    wit_bindgen::generate!({
        world: "oauth-auth",
        generate_all,
    });
}

mod config;
mod error;
mod helpers;
mod jwks;
mod jwt;
mod oauth;
mod policy;

use bindings::exports::wasmcp::mcp_v20250618::server_auth::{Guest, HttpContext};
use bindings::wasmcp::mcp_v20250618::mcp::{ClientMessage, Session};
use bindings::wasmcp::oauth::types::{Jwt, JwtClaims};
use std::collections::HashMap;
use std::sync::OnceLock;

static CONFIG: OnceLock<config::Config> = OnceLock::new();

/// Get or initialize configuration
fn get_config() -> &'static config::Config {
    CONFIG.get_or_init(|| {
        config::Config::load().unwrap_or_else(|e| {
            panic!("Failed to load configuration: {e}");
        })
    })
}

struct Component;

/// Fetch userinfo from the OIDC userinfo endpoint
fn fetch_userinfo(
    issuer: &str,
    access_token: &str,
) -> Result<HashMap<String, serde_json::Value>, String> {
    use bindings::wasi::http::outgoing_handler;
    use bindings::wasi::http::types::{Fields, Method, OutgoingRequest, Scheme};
    use bindings::wasi::io::poll;
    use bindings::wasi::io::streams::StreamError;

    // Construct userinfo URL (standard OIDC endpoint)
    let userinfo_url = format!("{}/oauth2/userinfo", issuer.trim_end_matches('/'));

    eprintln!("[oauth-auth] Fetching userinfo from: {}", userinfo_url);

    // Parse URL
    let url = userinfo_url
        .parse::<url::Url>()
        .map_err(|e| format!("Invalid userinfo URL: {}", e))?;

    let scheme = match url.scheme() {
        "https" => Scheme::Https,
        "http" => Scheme::Http,
        _ => return Err("Invalid URL scheme".to_string()),
    };

    let authority = url
        .host_str()
        .ok_or_else(|| "No host in URL".to_string())?
        .to_string();

    let path = if url.query().is_some() {
        format!("{}?{}", url.path(), url.query().unwrap())
    } else {
        url.path().to_string()
    };

    // Create headers with Authorization Bearer token
    let headers = Fields::new();
    let auth_value = format!("Bearer {}", access_token);
    headers
        .append("Authorization", auth_value.as_bytes())
        .map_err(|_| "Failed to set Authorization header".to_string())?;

    // Create request with headers
    let request = OutgoingRequest::new(headers);
    request
        .set_scheme(Some(&scheme))
        .map_err(|_| "Failed to set scheme".to_string())?;
    request
        .set_authority(Some(&authority))
        .map_err(|_| "Failed to set authority".to_string())?;
    request
        .set_path_with_query(Some(&path))
        .map_err(|_| "Failed to set path".to_string())?;
    request
        .set_method(&Method::Get)
        .map_err(|_| "Failed to set method".to_string())?;

    // Send request
    let future_response = outgoing_handler::handle(request, None)
        .map_err(|e| format!("Failed to send request: {:?}", e))?;

    // Poll for response (use correct polling pattern)
    let pollable = future_response.subscribe();
    poll::poll(&[&pollable]);
    drop(pollable);

    // Get response
    let incoming_response = future_response
        .get()
        .ok_or_else(|| "Response not ready after poll".to_string())?
        .map_err(|e| format!("Request failed: {:?}", e))?
        .map_err(|e| format!("HTTP error: {:?}", e))?;

    let status = incoming_response.status();

    if status != 200 {
        return Err(format!("Userinfo request failed with status: {}", status));
    }

    // Read response body
    let incoming_body = incoming_response
        .consume()
        .map_err(|_| "Failed to get response body".to_string())?;

    let input_stream = incoming_body
        .stream()
        .map_err(|_| "Failed to get response stream".to_string())?;

    let mut body_bytes = Vec::new();
    loop {
        match input_stream.blocking_read(4096) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    break;
                }
                body_bytes.extend_from_slice(&chunk);
            }
            Err(StreamError::Closed) => break,
            Err(e) => return Err(format!("Failed to read response: {:?}", e)),
        }
    }

    // Parse JSON response
    let body_str =
        String::from_utf8(body_bytes).map_err(|e| format!("Invalid UTF-8 in response: {}", e))?;

    let userinfo: HashMap<String, serde_json::Value> = serde_json::from_str(&body_str)
        .map_err(|e| format!("Failed to parse userinfo JSON: {}", e))?;

    Ok(userinfo)
}

impl Guest for Component {
    /// Decode and validate a JWT token
    fn decode(jwt_bytes: Jwt) -> Result<JwtClaims, ()> {
        // Convert JWT bytes to string
        let token_str = std::str::from_utf8(&jwt_bytes).map_err(|_| ())?;

        // Get configuration
        let config = get_config();

        // Verify JWT - returns JwtClaims directly!
        let jwt_claims = jwt::verify(token_str, &config.provider).map_err(|_| ())?;

        // Log entire decoded JWT structure
        eprintln!("[oauth-auth] Decoded JWT: {:#?}", jwt_claims);

        // Try to fetch userinfo to see additional claims
        if !config.provider.issuer.is_empty() {
            match fetch_userinfo(&config.provider.issuer, token_str) {
                Ok(userinfo) => {
                    eprintln!("[oauth-auth] Userinfo response: {:#?}", userinfo);
                }
                Err(e) => {
                    eprintln!("[oauth-auth] Failed to fetch userinfo: {}", e);
                }
            }
        }

        // Return JwtClaims directly - NO conversion needed!
        Ok(jwt_claims)
    }

    /// Authorize a request based on claims
    fn authorize(
        request: ClientMessage,
        claims: JwtClaims,
        session: Option<Session>,
        http_context: Option<HttpContext>,
    ) -> bool {
        let config = get_config();

        // If policy is configured, use policy engine
        if let Some(ref policy_str) = config.policy {
            // Create policy engine for this evaluation
            // Note: Regorous engine is not Send/Sync, so we create it per-request
            if let Ok(mut engine) = policy::PolicyEngine::new_with_policy_and_data(
                policy_str,
                config.policy_data.as_deref(),
            ) {
                // Pass JwtClaims and HTTP context to policy engine
                if let Ok(allowed) =
                    engine.evaluate(&claims, &request, session.as_ref(), http_context.as_ref())
                {
                    return allowed;
                }
            }
            // If policy evaluation fails, deny
            return false;
        }

        // Fallback: Simple scope-based authorization
        let Some(ref required_scopes) = config.provider.required_scopes else {
            return true;
        };

        // Use structured scopes directly - NO extraction needed!
        for required_scope in required_scopes {
            if !claims.scopes.contains(required_scope) {
                return false;
            }
        }

        true
    }
}

// === OAuth Interface Implementations ===

// Errors interface
impl bindings::exports::wasmcp::oauth::errors::Guest for Component {
    fn error_code_to_string(code: bindings::exports::wasmcp::oauth::errors::ErrorCode) -> String {
        oauth::errors::error_code_to_string(&code)
    }

    fn parse_error_code(
        error_string: String,
    ) -> Option<bindings::exports::wasmcp::oauth::errors::ErrorCode> {
        oauth::errors::parse_error_code(&error_string)
    }

    fn create_bearer_challenge(
        realm: Option<String>,
        error: Option<bindings::exports::wasmcp::oauth::errors::ErrorCode>,
        error_description: Option<String>,
        scope: Vec<String>,
    ) -> String {
        oauth::errors::create_bearer_challenge(realm, error, error_description, scope)
    }

    fn create_token_error_response(
        error: bindings::exports::wasmcp::oauth::errors::OauthError,
    ) -> String {
        oauth::errors::create_token_error_response(&error)
    }
}

// Bearer interface
impl bindings::exports::wasmcp::oauth::bearer::Guest for Component {
    fn extract_bearer_token(
        headers: Vec<(String, String)>,
        body_params: Option<Vec<(String, String)>>,
        query_params: Option<Vec<(String, String)>>,
    ) -> Result<
        (
            String,
            bindings::exports::wasmcp::oauth::bearer::BearerMethod,
        ),
        bindings::exports::wasmcp::oauth::errors::OauthError,
    > {
        oauth::bearer::extract_bearer_token(headers, body_params, query_params)
    }

    fn is_method_allowed(
        method: bindings::exports::wasmcp::oauth::bearer::BearerMethod,
        allowed_methods: Vec<bindings::exports::wasmcp::oauth::bearer::BearerMethod>,
    ) -> bool {
        oauth::bearer::is_method_allowed(&method, &allowed_methods)
    }

    fn is_valid_bearer_token_format(token: String) -> bool {
        oauth::bearer::is_valid_bearer_token_format(&token)
    }
}

// Introspection interface
impl bindings::exports::wasmcp::oauth::introspection::Guest for Component {
    fn to_jwt_claims(
        response: bindings::exports::wasmcp::oauth::introspection::IntrospectionResponse,
    ) -> Option<bindings::wasmcp::oauth::types::JwtClaims> {
        oauth::introspection::to_jwt_claims(&response)
    }

    fn introspect_token(
        introspection_endpoint: String,
        request: bindings::exports::wasmcp::oauth::introspection::IntrospectionRequest,
        client_credentials: (String, String),
    ) -> Result<
        bindings::exports::wasmcp::oauth::introspection::IntrospectionResponse,
        bindings::exports::wasmcp::oauth::errors::OauthError,
    > {
        oauth::introspection::introspect_token(
            &introspection_endpoint,
            &request,
            &client_credentials,
        )
    }
}

// Resource Metadata interface
impl bindings::exports::wasmcp::oauth::resource_metadata::Guest for Component {
    fn fetch_metadata(
        resource_url: String,
    ) -> Result<
        bindings::exports::wasmcp::oauth::resource_metadata::ProtectedResourceMetadata,
        String,
    > {
        oauth::resource_metadata::fetch_metadata(&resource_url)
    }

    fn validate_metadata(
        metadata: bindings::exports::wasmcp::oauth::resource_metadata::ProtectedResourceMetadata,
        expected_resource: String,
    ) -> Result<(), String> {
        oauth::resource_metadata::validate_metadata(&metadata, &expected_resource)
    }

    fn parse_www_authenticate_metadata(www_authenticate_header: String) -> Option<String> {
        oauth::resource_metadata::parse_www_authenticate_metadata(&www_authenticate_header)
    }
}

// JWT Claim Helpers interface
impl bindings::exports::wasmcp::oauth::helpers::Guest for Component {
    fn flatten_claims(claims: JwtClaims) -> Vec<(String, String)> {
        helpers::flatten_claims(&claims)
    }

    fn has_scope(claims: JwtClaims, scope: String) -> bool {
        helpers::has_scope(&claims, &scope)
    }

    fn has_any_scope(claims: JwtClaims, scopes: Vec<String>) -> bool {
        helpers::has_any_scope(&claims, &scopes)
    }

    fn has_all_scopes(claims: JwtClaims, scopes: Vec<String>) -> bool {
        helpers::has_all_scopes(&claims, &scopes)
    }

    fn get_claim(claims: JwtClaims, key: String) -> Option<String> {
        helpers::get_claim(&claims, &key)
    }

    fn has_audience(claims: JwtClaims, audience: String) -> bool {
        helpers::has_audience(&claims, &audience)
    }

    fn is_expired(claims: JwtClaims, clock_skew_seconds: Option<u64>) -> bool {
        helpers::is_expired(&claims, clock_skew_seconds)
    }

    fn is_valid_time(claims: JwtClaims, clock_skew_seconds: Option<u64>) -> bool {
        helpers::is_valid_time(&claims, clock_skew_seconds)
    }

    fn get_subject(claims: JwtClaims) -> String {
        helpers::get_subject(&claims)
    }

    fn get_issuer(claims: JwtClaims) -> Option<String> {
        helpers::get_issuer(&claims)
    }

    fn get_scopes(claims: JwtClaims) -> Vec<String> {
        helpers::get_scopes(&claims)
    }

    fn get_audiences(claims: JwtClaims) -> Vec<String> {
        helpers::get_audiences(&claims)
    }
}

bindings::export!(Component with_types_in bindings);
