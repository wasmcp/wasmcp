use crate::bindings::exports::fastertools::mcp::oauth_discovery::{
    ResourceMetadata, ServerMetadata,
};
use crate::config::{get_config, ConfigKeys};

/// Get OAuth 2.0 Protected Resource Metadata
/// This should be customized based on deployment configuration
pub fn get_resource_metadata() -> ResourceMetadata {
    // Get values from WASI config runtime
    let resource_url = get_config(ConfigKeys::RESOURCE_URL)
        .unwrap_or_else(|| "https://mcp.example.com".to_string());
    
    // Use JWT issuer as the authorization server (AuthKit domain)
    let auth_server = get_config(ConfigKeys::EXPECTED_ISSUER)
        .or_else(|| get_config(ConfigKeys::AUTH_SERVER))
        .unwrap_or_else(|| "https://auth.example.com".to_string());
    
    ResourceMetadata {
        resource_url,
        authorization_servers: vec![auth_server],
        scopes_supported: Some(vec![
            "mcp:tools:read".to_string(),
            "mcp:tools:write".to_string(),
            "mcp:resources:read".to_string(),
            "mcp:resources:write".to_string(),
            "mcp:prompts:read".to_string(),
        ]),
        bearer_methods_supported: Some(vec!["header".to_string()]),
        resource_documentation: Some("https://modelcontextprotocol.io/docs".to_string()),
    }
}

/// Get OAuth 2.0 Authorization Server Metadata
/// This provides discovery information about the authorization server
pub fn get_server_metadata() -> ServerMetadata {
    // Use JWT issuer as the base for OAuth endpoints (AuthKit domain)
    // This allows using a single config value for both JWT validation and OAuth discovery
    let issuer = get_config(ConfigKeys::EXPECTED_ISSUER)
        .or_else(|| get_config(ConfigKeys::AUTH_ISSUER))
        .unwrap_or_else(|| "https://auth.example.com".to_string());
    
    // AuthKit uses standard OAuth 2.0 endpoints
    let auth_endpoint = get_config(ConfigKeys::AUTH_ENDPOINT)
        .unwrap_or_else(|| format!("{}/oauth2/authorize", issuer));
    
    let token_endpoint = get_config(ConfigKeys::TOKEN_ENDPOINT)
        .unwrap_or_else(|| format!("{}/oauth2/token", issuer));
    
    // Use the configured JWKS URI or construct from issuer
    let jwks_uri = get_config(ConfigKeys::JWKS_URI)
        .unwrap_or_else(|| format!("{}/oauth2/jwks", issuer));
    
    let registration_endpoint = get_config(ConfigKeys::REGISTRATION_ENDPOINT)
        .or_else(|| Some(format!("{}/oauth2/register", issuer)));
    
    ServerMetadata {
        issuer,
        authorization_endpoint: auth_endpoint,
        token_endpoint,
        jwks_uri,
        response_types_supported: vec![
            "code".to_string(),
            "code id_token".to_string(),
        ],
        grant_types_supported: vec![
            "authorization_code".to_string(),
            "refresh_token".to_string(),
        ],
        code_challenge_methods_supported: vec![
            "S256".to_string(),
            "plain".to_string(),
        ],
        scopes_supported: Some(vec![
            "openid".to_string(),
            "profile".to_string(),
            "email".to_string(),
            "mcp:tools:read".to_string(),
            "mcp:tools:write".to_string(),
            "mcp:resources:read".to_string(),
            "mcp:resources:write".to_string(),
            "mcp:prompts:read".to_string(),
        ]),
        token_endpoint_auth_methods_supported: Some(vec![
            "client_secret_basic".to_string(),
            "client_secret_post".to_string(),
            "none".to_string(),
        ]),
        service_documentation: Some("https://modelcontextprotocol.io/docs".to_string()),
        registration_endpoint,
    }
}