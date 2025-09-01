use thiserror::Error;

/// Authorization error types
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    
    #[error("Token expired")]
    ExpiredToken,
    
    #[error("Invalid issuer")]
    InvalidIssuer,
    
    #[error("Invalid audience")]
    InvalidAudience,
    
    #[error("Invalid signature")]
    InvalidSignature,
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Insufficient scope: {0}")]
    InsufficientScope(String),
    
    #[error("Policy denied: {0}")]
    PolicyDenied(String),
}