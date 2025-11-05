//! Configuration for transport layer
//!
//! Environment variables:
//! - `MCP_SESSION_ENABLED`: "true"/"false" (default: "false") - Enable session support
//! - `MCP_SESSION_BUCKET`: Bucket name (default: "") - KV bucket for sessions
//! - `MCP_SSE_BUFFER`: "true"/"false" (default: "true") - Buffer SSE messages before sending (safer, works on Fermyon Cloud)

use crate::bindings::wasi::cli::environment::get_environment;
use std::collections::HashMap;

/// Session configuration from environment variables
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub enabled: bool,
    pub bucket_name: String,
    pub sse_buffer: bool,
}

impl SessionConfig {
    /// Load session configuration from environment variables
    ///
    /// - `MCP_SESSION_ENABLED`: "true"/"false" (case-insensitive, default: false)
    /// - `MCP_SESSION_BUCKET`: Bucket name (default: empty string)
    /// - `MCP_SSE_BUFFER`: "true"/"false" (case-insensitive, default: true)
    pub fn from_env() -> Self {
        let env_vars = get_environment();
        let env_map: HashMap<String, String> = env_vars.into_iter().collect();

        let enabled = env_map
            .get("MCP_SESSION_ENABLED")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);

        let bucket_name = env_map
            .get("MCP_SESSION_BUCKET")
            .cloned()
            .unwrap_or_default();

        let sse_buffer = env_map
            .get("MCP_SSE_BUFFER")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(true); // Default to buffered mode (safe everywhere)

        SessionConfig {
            enabled,
            bucket_name,
            sse_buffer,
        }
    }

    /// Get bucket name, returning default if empty
    ///
    /// Returns the configured bucket name, or the default bucket ("") if not configured.
    pub fn get_bucket(&self) -> &str {
        if self.bucket_name.is_empty() {
            crate::http::DEFAULT_SESSION_BUCKET
        } else {
            &self.bucket_name
        }
    }
}
