//! Configuration for transport layer
//!
//! Environment variables:
//! - `MCP_SESSION_ENABLED`: "true"/"false" (default: "false") - Enable session support
//! - `MCP_SESSION_BUCKET`: Bucket name (default: "") - KV bucket for sessions
//! - `MCP_SERVER_MODE`: "sse"/"sse_buffer"/"json" (default: "sse_buffer") - Server response mode

use crate::bindings::wasi::cli::environment::get_environment;
use std::collections::HashMap;

/// Server mode for handling responses
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerMode {
    /// True streaming SSE - immediate writes
    Sse,
    /// Buffered SSE - accumulate and flush once (default)
    SseBuffer,
    /// Plain JSON-RPC - single response, no SSE
    Json,
}

impl Default for ServerMode {
    fn default() -> Self {
        ServerMode::SseBuffer
    }
}

/// Session configuration from environment variables
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub enabled: bool,
    pub bucket_name: String,
    pub mode: ServerMode,
}

impl SessionConfig {
    /// Load session configuration from environment variables
    ///
    /// - `MCP_SESSION_ENABLED`: "true"/"false" (case-insensitive, default: false)
    /// - `MCP_SESSION_BUCKET`: Bucket name (default: empty string)
    /// - `MCP_SERVER_MODE`: "sse"/"sse_buffer"/"json" (case-insensitive, default: sse_buffer)
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

        let mode = env_map
            .get("MCP_SERVER_MODE")
            .and_then(|v| match v.to_lowercase().as_str() {
                "sse" => Some(ServerMode::Sse),
                "sse_buffer" => Some(ServerMode::SseBuffer),
                "json" => Some(ServerMode::Json),
                _ => None,
            })
            .unwrap_or(ServerMode::SseBuffer); // Default to buffered mode (safe everywhere)

        SessionConfig {
            enabled,
            bucket_name,
            mode,
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
