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
    /// - `MCP_SESSION_BUCKET`: Bucket name (default: "mcp-sessions")
    /// - `MCP_RUNTIME`: "wasmtime" uses empty bucket name, others use configured name
    pub fn from_env() -> Self {
        let env_vars = get_environment();
        let env_map: std::collections::HashMap<String, String> = env_vars.into_iter().collect();

        eprintln!("[SESSION_CONFIG] Environment variables:");
        for (key, value) in &env_map {
            if key.starts_with("MCP_") {
                eprintln!("[SESSION_CONFIG]   {}={}", key, value);
            }
        }

        let enabled = env_map
            .get("MCP_SESSION_ENABLED")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);

        let runtime = env_map
            .get("MCP_RUNTIME")
            .map(|v| v.to_lowercase())
            .unwrap_or_default();

        // Wasmtime only supports empty string as bucket name
        let bucket_name = if runtime == "wasmtime" {
            eprintln!("[SESSION_CONFIG] Runtime is wasmtime, using empty bucket name");
            String::new()
        } else {
            env_map
                .get("MCP_SESSION_BUCKET")
                .cloned()
                .unwrap_or_else(|| "mcp-sessions".to_string())
        };

        eprintln!("[SESSION_CONFIG] Final config: enabled={}, bucket_name='{}', runtime={}",
                  enabled, bucket_name, runtime);

        SessionConfig {
            enabled,
            bucket_name,
        }
    }
}
