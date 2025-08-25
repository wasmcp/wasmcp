//! Configuration access using WASI Config interfaces

// Use the actual module path that wit-bindgen generates
use crate::wit::wasi::config;

use anyhow::Result;

/// Get a configuration value by key
pub fn get(key: &str) -> Result<Option<String>> {
    config::store::get(key)
        .map_err(|e| anyhow::anyhow!("Failed to get config: {:?}", e))
}

/// Get all configuration values
pub fn get_all() -> Result<Vec<(String, String)>> {
    config::store::get_all()
        .map_err(|e| anyhow::anyhow!("Failed to get all config: {:?}", e))
}