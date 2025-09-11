use thiserror::Error;

#[derive(Error, Debug)]
pub enum McpError {
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    #[error("Authentication failed: {0}")]
    Auth(#[from] AuthError),
    
    #[error("Invalid JWT: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    
    #[error("Policy evaluation failed: {0}")]
    Policy(String),
    
    #[error("HTTP request failed: {0}")]
    Http(String),
    
    #[error("Cache operation failed: {0}")]
    Cache(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Runtime error: {0}")]
    Runtime(String),
}

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid token")]
    InvalidToken,
    
    #[error("Token expired")]
    TokenExpired,
    
    #[error("Missing required claim: {0}")]
    MissingClaim(String),
    
    #[error("Policy denied access: {0}")]
    PolicyDenied(String),
    
    #[error("JWKS discovery failed: {0}")]
    JwksDiscovery(String),
    
    #[error("Invalid signature")]
    InvalidSignature,
}