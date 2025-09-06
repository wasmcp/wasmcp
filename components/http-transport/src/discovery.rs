use crate::auth_types::{ResourceMetadata, ServerMetadata};
use crate::bindings::fastertools::mcp::authorization_types::ProviderAuthConfig;

/// Get OAuth 2.0 Protected Resource Metadata
/// Uses the provider's auth configuration to build the metadata
pub fn get_resource_metadata(
    provider_config: &ProviderAuthConfig,
    server_url: &str,
) -> ResourceMetadata {
    // Extract the authorization server URL from the JWKS URI
    // e.g., "https://divine-lion-50-staging.authkit.app/oauth2/jwks" -> "https://divine-lion-50-staging.authkit.app"
    let auth_server = provider_config
        .jwks_uri
        .rsplit_once("/oauth2/jwks")
        .or_else(|| {
            provider_config
                .jwks_uri
                .rsplit_once("/.well-known/jwks.json")
        })
        .map(|(base, _)| base.to_string())
        .unwrap_or_else(|| provider_config.expected_issuer.clone());

    ResourceMetadata {
        resource_url: server_url.to_string(),
        authorization_servers: vec![auth_server],
        scopes_supported: None, // Let the authorization server define its own scopes
        bearer_methods_supported: Some(vec!["header".to_string()]),
        resource_documentation: Some("https://modelcontextprotocol.io/docs".to_string()),
    }
}

/// Get OAuth 2.0 Authorization Server Metadata  
/// Uses the provider's auth configuration to build the metadata
pub fn get_server_metadata(provider_config: &ProviderAuthConfig) -> ServerMetadata {
    // Extract the authorization server URL from the JWKS URI or use issuer
    let auth_server = provider_config
        .jwks_uri
        .rsplit_once("/oauth2/jwks")
        .or_else(|| {
            provider_config
                .jwks_uri
                .rsplit_once("/.well-known/jwks.json")
        })
        .map(|(base, _)| base.to_string())
        .unwrap_or_else(|| provider_config.expected_issuer.clone());

    ServerMetadata {
        issuer: provider_config.expected_issuer.clone(),
        authorization_endpoint: format!("{auth_server}/oauth2/authorize"),
        token_endpoint: format!("{auth_server}/oauth2/token"),
        jwks_uri: provider_config.jwks_uri.clone(),
        response_types_supported: vec!["code".to_string()],
        grant_types_supported: vec![
            "authorization_code".to_string(),
            "refresh_token".to_string(),
        ],
        code_challenge_methods_supported: vec!["S256".to_string()],
        scopes_supported: Some(vec![
            "openid".to_string(),
            "profile".to_string(),
            "email".to_string(),
            "offline_access".to_string(),
        ]),
        token_endpoint_auth_methods_supported: Some(vec![
            "none".to_string(),
            "client_secret_post".to_string(),
            "client_secret_basic".to_string(),
        ]),
        service_documentation: Some("https://modelcontextprotocol.io/docs".to_string()),
        registration_endpoint: Some(format!("{auth_server}/oauth2/register")),
    }
}
