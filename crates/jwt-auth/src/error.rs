//! Error types for JWT authentication

use std::fmt;

/// Result type alias for auth operations
pub type Result<T> = std::result::Result<T, AuthError>;

/// Authentication and authorization errors
#[derive(Debug)]
pub enum AuthError {
    /// Missing authorization or invalid format
    Unauthorized(String),

    /// Invalid token format or content
    InvalidToken(String),

    /// Token has expired
    ExpiredToken,

    /// Token issuer doesn't match expected
    InvalidIssuer,

    /// Token audience doesn't match expected
    InvalidAudience,

    /// Token signature verification failed
    InvalidSignature,

    /// Configuration error
    Configuration(String),

    /// Internal server error
    Internal(String),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unauthorized(msg) => write!(f, "Unauthorized: {msg}"),
            Self::InvalidToken(msg) => write!(f, "Invalid token: {msg}"),
            Self::ExpiredToken => write!(f, "Token has expired"),
            Self::InvalidIssuer => write!(f, "Invalid issuer"),
            Self::InvalidAudience => write!(f, "Invalid audience"),
            Self::InvalidSignature => write!(f, "Invalid signature"),
            Self::Configuration(msg) => write!(f, "Configuration error: {msg}"),
            Self::Internal(msg) => write!(f, "Internal error: {msg}"),
        }
    }
}

impl std::error::Error for AuthError {}

impl From<serde_json::Error> for AuthError {
    fn from(err: serde_json::Error) -> Self {
        Self::Internal(format!("JSON error: {err}"))
    }
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind;

        match err.kind() {
            ErrorKind::ExpiredSignature => Self::ExpiredToken,
            ErrorKind::InvalidSignature => Self::InvalidSignature,
            ErrorKind::InvalidIssuer => Self::InvalidIssuer,
            ErrorKind::InvalidAudience => Self::InvalidAudience,
            ErrorKind::InvalidToken => Self::InvalidToken("JWT validation failed".to_string()),
            _ => Self::InvalidToken(err.to_string()),
        }
    }
}
