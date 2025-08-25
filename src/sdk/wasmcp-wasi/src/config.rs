//! Configuration access using WASI Config interfaces

use crate::wit::wasi::config::store;
use anyhow::Result;

/// Get a configuration value by key
pub fn get(key: &str) -> Result<Option<String>> {
    store::get(key)
        .map_err(|e| anyhow::anyhow!("Failed to get config: {:?}", e))
}

/// Get all configuration values
pub fn get_all() -> Result<Vec<(String, String)>> {
    store::get_all()
        .map_err(|e| anyhow::anyhow!("Failed to get all config: {:?}", e))
}