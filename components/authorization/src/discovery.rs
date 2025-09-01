use crate::bindings::exports::fastertools::mcp::oauth_discovery::{
    ResourceMetadata, ServerMetadata,
};

/// Get OAuth 2.0 Protected Resource Metadata
/// This should be customized based on deployment configuration
pub fn get_resource_metadata() -> ResourceMetadata {
    // These values would typically come from environment variables or configuration
    let resource_url = std::env::var("MCP_RESOURCE_URL")
        .unwrap_or_else(|_| "https://mcp.example.com".to_string());
    
    let auth_server = std::env::var("MCP_AUTH_SERVER")
        .unwrap_or_else(|_| "https://auth.example.com".to_string());
    
    ResourceMetadata {
        resource: resource_url,
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
    // These values would typically come from environment variables or configuration
    let issuer = std::env::var("MCP_AUTH_ISSUER")
        .unwrap_or_else(|_| "https://auth.example.com".to_string());
    
    let auth_endpoint = std::env::var("MCP_AUTH_ENDPOINT")
        .unwrap_or_else(|_| format!("{}/authorize", issuer));
    
    let token_endpoint = std::env::var("MCP_TOKEN_ENDPOINT")
        .unwrap_or_else(|_| format!("{}/token", issuer));
    
    let jwks_uri = std::env::var("MCP_JWKS_URI")
        .unwrap_or_else(|_| format!("{}/.well-known/jwks.json", issuer));
    
    let registration_endpoint = std::env::var("MCP_REGISTRATION_ENDPOINT")
        .ok()
        .or_else(|| Some(format!("{}/register", issuer)));
    
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