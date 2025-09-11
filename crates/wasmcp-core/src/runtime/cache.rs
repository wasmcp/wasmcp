use async_trait::async_trait;
use std::time::Duration;
use crate::error::McpError;

/// Cache provider abstraction for storing temporary data
/// Used for caching JWKS, policy decisions, and other ephemeral data
#[async_trait]
pub trait CacheProvider: Send + Sync {
    /// Get a value from the cache by key
    async fn get(&self, key: &str) -> Option<Vec<u8>>;
    
    /// Set a value in the cache with a time-to-live
    async fn set(&self, key: &str, value: Vec<u8>, ttl: Duration) -> Result<(), McpError>;
    
    /// Delete a value from the cache
    async fn delete(&self, key: &str) -> Result<(), McpError>;
}