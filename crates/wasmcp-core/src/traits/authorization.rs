use crate::{ProviderAuthConfig, McpError};

/// Defines the contract for MCP authorization providers.
///
/// This trait provides authorization capabilities at the transport level, enabling
/// MCP clients to make requests to restricted MCP servers on behalf of resource owners.
pub trait McpAuthorizationHandler {
    /// Get provider's auth configuration
    /// The transport should enforce authorization
    fn get_auth_config(&self) -> Result<Option<ProviderAuthConfig>, McpError>;

    /// Get cached JWKS for a given URI (optional - return None if not cached or not implemented)
    /// Allows providers to implement JWKS caching via WASI-KV or other persistence mechanisms
    /// The transport will call this before fetching from jwks-uri to check for cached keys
    fn jwks_cache_get(&self, jwks_uri: String) -> Result<Option<String>, McpError>;

    /// Cache JWKS for a given URI (optional - no-op if caching not implemented)
    /// The transport calls this after successfully fetching JWKS from jwks-uri
    /// Providers can implement caching via WASI-KV or other persistence mechanisms
    /// The jwks parameter contains the raw JWKS JSON string to cache
    fn jwks_cache_set(&self, jwks_uri: String, jwks: String) -> Result<(), McpError>;
}