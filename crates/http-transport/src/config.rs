//! Configuration for HTTP transport, loaded from environment variables

use crate::bindings::wasi::cli::environment::get_environment;

/// Session configuration from environment variables
pub struct SessionConfig {
    /// Whether session support is enabled
    pub enabled: bool,
    /// KV bucket name for session storage
    pub bucket_name: String,
}

impl SessionConfig {
    /// Load configuration from environment variables
    ///
    /// Environment variables:
    /// - `MCP_SESSION_ENABLED`: "true"/"false" (default: "false")
    /// - `MCP_SESSION_BUCKET`: Bucket name (default: "")
    ///   - If set, must be "default"
    ///   - If not set, defaults to empty string (wasmtime's default bucket)
    ///   - Ignored if MCP_SESSION_ENABLED is not "true"
    pub fn from_env() -> Self {
        let env_vars = get_environment();
        let env_map: std::collections::HashMap<String, String> = env_vars.into_iter().collect();

        let enabled = env_map
            .get("MCP_SESSION_ENABLED")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);

        // Default to empty string if MCP_SESSION_BUCKET not set
        // If set, it must be "default" (or empty string for wasmtime's default bucket)
        let bucket_name = env_map
            .get("MCP_SESSION_BUCKET")
            .cloned()
            .unwrap_or_default();

        SessionConfig {
            enabled,
            bucket_name,
        }
    }
}
