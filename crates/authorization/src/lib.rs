//! OAuth Authentication Component
//!
//! Exports the server-auth interface for OAuth 2.1 Resource Server authentication and authorization.
//! Supports JWT validation, token introspection, and policy-based authorization.

mod bindings {
    wit_bindgen::generate!({
        world: "authorization",
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
mod utils;

use bindings::exports::wasmcp::mcp_v20250618::server_auth::{Guest, HttpContext};
use bindings::wasmcp::auth::types::{Jwt, JwtClaims};
use bindings::wasmcp::mcp_v20250618::mcp::{ClientMessage, Session};
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
    fn decode(jwt_bytes: Jwt) -> Result<JwtClaims, ()> {
        // Convert JWT bytes to string
        let token_str = std::str::from_utf8(&jwt_bytes).map_err(|_| ())?;

        // Get configuration
        let config = get_config();

        // Verify JWT - returns JwtClaims directly!
        let jwt_claims = jwt::verify(token_str, &config.provider).map_err(|_| ())?;

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
impl bindings::exports::wasmcp::auth::errors::Guest for Component {
    fn error_code_to_string(code: bindings::exports::wasmcp::auth::errors::ErrorCode) -> String {
        oauth::errors::error_code_to_string(&code)
    }

    fn parse_error_code(
        error_string: String,
    ) -> Option<bindings::exports::wasmcp::auth::errors::ErrorCode> {
        oauth::errors::parse_error_code(&error_string)
    }

    fn create_bearer_challenge(
        realm: Option<String>,
        error: Option<bindings::exports::wasmcp::auth::errors::ErrorCode>,
        error_description: Option<String>,
        scope: Vec<String>,
    ) -> String {
        oauth::errors::create_bearer_challenge(realm, error, error_description, scope)
    }

    fn create_token_error_response(
        error: bindings::exports::wasmcp::auth::errors::OauthError,
    ) -> String {
        oauth::errors::create_token_error_response(&error)
    }
}

// Bearer interface
impl bindings::exports::wasmcp::auth::bearer::Guest for Component {
    fn extract_bearer_token(
        headers: Vec<(String, String)>,
        body_params: Option<Vec<(String, String)>>,
        query_params: Option<Vec<(String, String)>>,
    ) -> Result<
        (
            String,
            bindings::exports::wasmcp::auth::bearer::BearerMethod,
        ),
        bindings::exports::wasmcp::auth::errors::OauthError,
    > {
        oauth::bearer::extract_bearer_token(headers, body_params, query_params)
    }

    fn is_method_allowed(
        method: bindings::exports::wasmcp::auth::bearer::BearerMethod,
        allowed_methods: Vec<bindings::exports::wasmcp::auth::bearer::BearerMethod>,
    ) -> bool {
        oauth::bearer::is_method_allowed(&method, &allowed_methods)
    }

    fn is_valid_bearer_token_format(token: String) -> bool {
        oauth::bearer::is_valid_bearer_token_format(&token)
    }
}

// Introspection interface
impl bindings::exports::wasmcp::auth::introspection::Guest for Component {
    fn to_jwt_claims(
        response: bindings::exports::wasmcp::auth::introspection::IntrospectionResponse,
    ) -> Option<bindings::wasmcp::auth::types::JwtClaims> {
        oauth::introspection::to_jwt_claims(&response)
    }

    fn introspect_token(
        introspection_endpoint: String,
        request: bindings::exports::wasmcp::auth::introspection::IntrospectionRequest,
        client_credentials: (String, String),
    ) -> Result<
        bindings::exports::wasmcp::auth::introspection::IntrospectionResponse,
        bindings::exports::wasmcp::auth::errors::OauthError,
    > {
        oauth::introspection::introspect_token(
            &introspection_endpoint,
            &request,
            &client_credentials,
        )
    }
}

// JWT Claim Helpers interface
impl bindings::exports::wasmcp::auth::helpers::Guest for Component {
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
