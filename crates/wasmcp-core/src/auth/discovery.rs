use crate::error::McpError;
use serde::{Deserialize, Serialize};

/// OAuth 2.0 discovery endpoint handler
pub struct OAuthDiscovery {
    issuer: String,
    jwks_uri: String,
    authorization_endpoint: Option<String>,
    token_endpoint: Option<String>,
}

impl OAuthDiscovery {
    pub fn new(issuer: String, jwks_uri: String) -> Self {
        Self {
            issuer,
            jwks_uri,
            authorization_endpoint: None,
            token_endpoint: None,
        }
    }

    pub fn with_authorization_endpoint(mut self, endpoint: String) -> Self {
        self.authorization_endpoint = Some(endpoint);
        self
    }

    pub fn with_token_endpoint(mut self, endpoint: String) -> Self {
        self.token_endpoint = Some(endpoint);
        self
    }

    /// Generate OAuth 2.0 discovery metadata
    pub fn get_metadata(&self) -> Result<OAuthMetadata, McpError> {
        Ok(OAuthMetadata {
            issuer: self.issuer.clone(),
            jwks_uri: self.jwks_uri.clone(),
            authorization_endpoint: self.authorization_endpoint.clone(),
            token_endpoint: self.token_endpoint.clone(),
            response_types_supported: vec!["code".to_string(), "token".to_string()],
            subject_types_supported: vec!["public".to_string()],
            id_token_signing_alg_values_supported: vec!["RS256".to_string()],
        })
    }

    /// Generate authorization server metadata (RFC 8414)
    pub fn get_authorization_server_metadata(&self) -> Result<AuthorizationServerMetadata, McpError> {
        Ok(AuthorizationServerMetadata {
            issuer: self.issuer.clone(),
            jwks_uri: self.jwks_uri.clone(),
            authorization_endpoint: self.authorization_endpoint.clone(),
            token_endpoint: self.token_endpoint.clone(),
            response_types_supported: vec!["code".to_string(), "token".to_string()],
            grant_types_supported: vec![
                "authorization_code".to_string(),
                "implicit".to_string(),
            ],
            code_challenge_methods_supported: vec!["S256".to_string()],
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthMetadata {
    pub issuer: String,
    pub jwks_uri: String,
    pub authorization_endpoint: Option<String>,
    pub token_endpoint: Option<String>,
    pub response_types_supported: Vec<String>,
    pub subject_types_supported: Vec<String>,
    pub id_token_signing_alg_values_supported: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationServerMetadata {
    pub issuer: String,
    pub jwks_uri: String,
    pub authorization_endpoint: Option<String>,
    pub token_endpoint: Option<String>,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<String>,
}