use async_trait::async_trait;
use crate::error::McpError;

/// HTTP client abstraction for making requests
/// Implementations must be Send + Sync for use across async boundaries
#[async_trait]
pub trait HttpClient: Send + Sync {
    /// Perform a GET request to the specified URL
    async fn get(&self, url: &str) -> Result<String, McpError>;
    
    /// Perform a POST request with the given body
    async fn post(&self, url: &str, body: &str) -> Result<String, McpError>;
}