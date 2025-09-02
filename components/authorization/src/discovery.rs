use crate::bindings::exports::fastertools::mcp::oauth_discovery::{
    ResourceMetadata, ServerMetadata,
};

/// Get OAuth 2.0 Protected Resource Metadata
/// Returns hardcoded values - the transport should provide actual values
pub fn get_resource_metadata() -> ResourceMetadata {
    ResourceMetadata {
        resource_url: "https://mcp.example.com".to_string(),
        authorization_servers: vec!["https://auth.example.com".to_string()],
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
/// Returns hardcoded values - the transport should provide actual values
pub fn get_server_metadata() -> ServerMetadata {
    let issuer = "https://auth.example.com".to_string();
    let auth_endpoint = format!("{}/oauth2/authorize", issuer);
    let token_endpoint = format!("{}/oauth2/token", issuer);
    let jwks_uri = format!("{}/oauth2/jwks", issuer);
    let registration_endpoint = Some(format!("{}/oauth2/register", issuer));
    
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