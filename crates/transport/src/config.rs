//! Configuration for transport layer
//!
//! Environment variables:
//! - `WASMCP_SESSION_ENABLED`: "true"/"false" (default: "false") - Enable session support
//! - `WASMCP_SESSION_BUCKET`: Bucket name (default: "") - KV bucket for sessions
//! - `WASMCP_DISABLE_SSE`: "true"/"false" (default: "false") - Use plain JSON instead of SSE for HTTP
//! - `WASMCP_AUTH_MODE`: "public"/"oauth" (default: "public") - Authentication mode
//! - `JWT_PUBLIC_KEY`: PEM-encoded public key (optional, alternative to JWT_JWKS_URI)
//! - `JWT_JWKS_URI`: JWKS endpoint URL (optional, alternative to JWT_PUBLIC_KEY)

use crate::bindings::wasi::cli::environment::get_environment;
use std::collections::HashMap;

/// Authentication mode for MCP server
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuthMode {
    /// Public server - no authentication required (default)
    #[default]
    Public,
    /// OAuth protected server - JWT required
    OAuth,
}

/// Transport configuration from environment variables
#[derive(Debug, Clone)]
pub struct TransportConfig {
    // Session configuration
    pub session_enabled: bool,
    pub session_bucket_name: String,

    // HTTP mode (SSE vs plain JSON)
    pub disable_sse: bool,

    // Authentication configuration
    pub auth_mode: AuthMode,
    pub jwt_configured: bool,
}

impl TransportConfig {
    /// Load transport configuration from environment variables
    ///
    /// Reads all configuration in a single pass:
    /// - `WASMCP_SESSION_ENABLED`: "true"/"false" (case-insensitive, default: false)
    /// - `WASMCP_SESSION_BUCKET`: Bucket name (default: empty string)
    /// - `WASMCP_DISABLE_SSE`: "true"/"false" (case-insensitive, default: false)
    /// - `WASMCP_AUTH_MODE`: "public"/"oauth" (case-insensitive, default: public)
    /// - `JWT_PUBLIC_KEY`: PEM public key (optional)
    /// - `JWT_JWKS_URI`: JWKS endpoint URL (optional)
    pub fn from_env() -> Self {
        let env_vars = get_environment();
        let env_map: HashMap<String, String> = env_vars.into_iter().collect();

        // Session configuration
        let session_enabled = env_map
            .get("WASMCP_SESSION_ENABLED")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);

        let session_bucket_name = env_map
            .get("WASMCP_SESSION_BUCKET")
            .cloned()
            .unwrap_or_default();

        // HTTP mode (SSE vs plain JSON)
        let disable_sse = env_map
            .get("WASMCP_DISABLE_SSE")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);

        // Authentication mode
        let auth_mode_str = env_map
            .get("WASMCP_AUTH_MODE")
            .map(|v| v.to_lowercase())
            .unwrap_or_else(|| "public".to_string());

        let auth_mode = match auth_mode_str.as_str() {
            "oauth" => AuthMode::OAuth,
            "public" => AuthMode::Public,
            _ => {
                eprintln!(
                    "[transport] WARNING: Invalid WASMCP_AUTH_MODE='{}', defaulting to 'public'. \
                     Valid values: 'public', 'oauth'",
                    auth_mode_str
                );
                AuthMode::Public
            }
        };

        // Check if JWT is configured
        let jwt_configured = env_map
            .get("JWT_PUBLIC_KEY")
            .filter(|v| !v.is_empty())
            .is_some()
            || env_map
                .get("JWT_JWKS_URI")
                .filter(|v| !v.is_empty())
                .is_some();

        TransportConfig {
            session_enabled,
            session_bucket_name,
            disable_sse,
            auth_mode,
            jwt_configured,
        }
    }

    /// Get session bucket name, returning default if empty
    ///
    /// Returns the configured bucket name, or the default bucket ("") if not configured.
    pub fn get_session_bucket(&self) -> &str {
        if self.session_bucket_name.is_empty() {
            crate::http::DEFAULT_SESSION_BUCKET
        } else {
            &self.session_bucket_name
        }
    }
}

