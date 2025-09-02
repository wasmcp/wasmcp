use crate::bindings::wasi::config::store;

/// Get configuration value from WASI config runtime
pub fn get_config(key: &str) -> Option<String> {
    match store::get(key) {
        Ok(Some(value)) => {
            eprintln!("Config: {} = {}", key, value);
            Some(value)
        }
        Ok(None) => {
            eprintln!("Config key '{}' not found", key);
            None
        }
        Err(e) => {
            eprintln!("Error getting config '{}': {:?}", key, e);
            None
        }
    }
}

/// Get required configuration value, panic if not found
pub fn get_required_config(key: &str) -> String {
    get_config(key).unwrap_or_else(|| {
        panic!("Required configuration '{}' not found", key);
    })
}

/// Configuration keys used by the authorization component
pub struct ConfigKeys;

impl ConfigKeys {
    // JWT validation configuration
    pub const EXPECTED_ISSUER: &'static str = "jwt.expected_issuer";
    pub const EXPECTED_AUDIENCE: &'static str = "jwt.expected_audience";
    pub const JWKS_URI: &'static str = "jwt.jwks_uri";
    pub const VALIDATION_LEEWAY: &'static str = "jwt.validation_leeway";
    
    // OAuth discovery configuration
    pub const RESOURCE_URL: &'static str = "oauth.resource_url";
    pub const AUTH_SERVER: &'static str = "oauth.auth_server";
    pub const AUTH_ISSUER: &'static str = "oauth.auth_issuer";
    pub const AUTH_ENDPOINT: &'static str = "oauth.auth_endpoint";
    pub const TOKEN_ENDPOINT: &'static str = "oauth.token_endpoint";
    pub const REGISTRATION_ENDPOINT: &'static str = "oauth.registration_endpoint";
    
    // Policy configuration
    pub const POLICY_MODE: &'static str = "policy.mode"; // "default", "rbac", "custom"
    pub const POLICY_PATH: &'static str = "policy.path"; // Path to custom policy file
}