//! JWT Authentication Component
//!
//! Exports the server-auth interface for JWT-based authentication and authorization.

mod bindings {
    wit_bindgen::generate!({
        world: "jwt-auth",
        generate_all,
    });
}

mod config;
mod error;
mod jwks;
mod jwt;
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

bindings::export!(Component with_types_in bindings);
