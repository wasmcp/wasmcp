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
mod jwks;
mod jwt;
mod oauth;
mod policy;

use bindings::exports::wasmcp::mcp_v20250618::server_auth::Guest;
use bindings::wasmcp::mcp_v20250618::mcp::{Claims, ClientMessage, Jwt, Session};
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

impl Guest for Component {
    /// Decode and validate a JWT token
    fn decode(jwt_bytes: Jwt) -> Result<Claims, ()> {
        // Convert JWT bytes to string
        let token_str = std::str::from_utf8(&jwt_bytes).map_err(|_| ())?;

        // Get configuration
        let config = get_config();

        // Verify JWT
        let token_info = jwt::verify(token_str, &config.provider).map_err(|_| ())?;

        // Convert claims to WIT format: list<tuple<string, string>>
        let claims: Vec<(String, String)> = token_info
            .claims
            .iter()
            .map(|(k, v)| {
                // Convert JSON values to strings
                let value_str = match v {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Null => "null".to_string(),
                    _ => serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string()),
                };
                (k.clone(), value_str)
            })
            .collect();

        Ok(claims)
    }

    /// Authorize a request based on claims
    fn authorize(request: ClientMessage, claims: Claims, session: Option<Session>) -> bool {
        let config = get_config();

        // If policy is configured, use policy engine
        if let Some(ref policy_str) = config.policy {
            // Create policy engine for this evaluation
            // Note: Regorous engine is not Send/Sync, so we create it per-request
            if let Ok(mut engine) = policy::PolicyEngine::new_with_policy_and_data(
                policy_str,
                config.policy_data.as_deref(),
            ) {
                // Convert claims back to TokenInfo for policy evaluation
                let token_info = jwt::TokenInfo {
                    sub: claims
                        .iter()
                        .find(|(k, _)| k == "sub")
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default(),
                    iss: claims
                        .iter()
                        .find(|(k, _)| k == "iss")
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default(),
                    scopes: extract_scopes_from_claims(&claims),
                    claims: claims
                        .iter()
                        .filter_map(|(k, v)| {
                            serde_json::from_str(v).ok().map(|val| (k.clone(), val))
                        })
                        .collect(),
                };

                if let Ok(allowed) = engine.evaluate(&token_info, &request, session.as_ref()) {
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

        let scopes = extract_scopes_from_claims(&claims);

        for required_scope in required_scopes {
            if !scopes.contains(required_scope) {
                return false;
            }
        }

        true
    }
}

/// Extract scopes from claims (looking for "scope" or "scp" claim)
fn extract_scopes_from_claims(claims: &Claims) -> Vec<String> {
    // Look for "scope" claim (OAuth2 standard)
    for (key, value) in claims {
        if key == "scope" {
            // Split space-separated scopes
            return value.split_whitespace().map(String::from).collect();
        }
    }

    // Look for "scp" claim (Microsoft style)
    for (key, value) in claims {
        if key == "scp" {
            // Try to parse as JSON array first, fall back to space-separated
            if let Ok(serde_json::Value::Array(scopes)) = serde_json::from_str(value) {
                return scopes
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
            // Fall back to space-separated
            return value.split_whitespace().map(String::from).collect();
        }
    }

    Vec::new()
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

// Helpers interface
impl bindings::exports::wasmcp::oauth::helpers::Guest for Component {
    fn get_claim(claims: bindings::wasmcp::oauth::types::JwtClaims, key: String) -> Option<String> {
        oauth::helpers::get_claim(&claims, &key)
    }

    fn has_scope(claims: bindings::wasmcp::oauth::types::JwtClaims, scope: String) -> bool {
        oauth::helpers::has_scope(&claims, &scope)
    }

    fn has_any_scope(
        claims: bindings::wasmcp::oauth::types::JwtClaims,
        scopes: Vec<String>,
    ) -> bool {
        oauth::helpers::has_any_scope(&claims, &scopes)
    }

    fn has_all_scopes(
        claims: bindings::wasmcp::oauth::types::JwtClaims,
        scopes: Vec<String>,
    ) -> bool {
        oauth::helpers::has_all_scopes(&claims, &scopes)
    }

    fn has_audience(claims: bindings::wasmcp::oauth::types::JwtClaims, audience: String) -> bool {
        oauth::helpers::has_audience(&claims, &audience)
    }

    fn is_expired(claims: bindings::wasmcp::oauth::types::JwtClaims) -> bool {
        oauth::helpers::is_expired(&claims)
    }

    fn is_valid_time(claims: bindings::wasmcp::oauth::types::JwtClaims) -> bool {
        oauth::helpers::is_valid_time(&claims)
    }

    fn get_subject(claims: bindings::wasmcp::oauth::types::JwtClaims) -> String {
        oauth::helpers::get_subject(&claims)
    }

    fn get_issuer(claims: bindings::wasmcp::oauth::types::JwtClaims) -> Option<String> {
        oauth::helpers::get_issuer(&claims)
    }

    fn get_scopes(claims: bindings::wasmcp::oauth::types::JwtClaims) -> Vec<String> {
        oauth::helpers::get_scopes(&claims)
    }
}

// Session Claims interface
impl bindings::exports::wasmcp::oauth::session_claims::Guest for Component {
    fn parse_claims(
        flat_claims: Vec<(String, String)>,
    ) -> Result<
        bindings::wasmcp::oauth::types::JwtClaims,
        bindings::exports::wasmcp::oauth::errors::OauthError,
    > {
        oauth::session_claims::parse_claims(flat_claims)
    }

    fn flatten_claims(claims: bindings::wasmcp::oauth::types::JwtClaims) -> Vec<(String, String)> {
        oauth::session_claims::flatten_claims(&claims)
    }

    fn has_claim(session_id: String, claim_key: String) -> Option<String> {
        oauth::session_claims::has_claim(&session_id, &claim_key)
    }

    fn has_session_scope(session_id: String, scope: String) -> bool {
        oauth::session_claims::has_session_scope(&session_id, &scope)
    }

    fn get_session_claims(session_id: String) -> Option<bindings::wasmcp::oauth::types::JwtClaims> {
        oauth::session_claims::get_session_claims(&session_id)
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

bindings::export!(Component with_types_in bindings);
