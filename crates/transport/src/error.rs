//! Transport error types
//!
//! Provides a unified error type for all transport-layer operations,
//! eliminating the need for repeated error handling boilerplate.

use crate::bindings::wasmcp::mcp_v20250618::mcp::ErrorCode;
use crate::bindings::wasmcp::mcp_v20250618::server_io::IoError;

/// Unified transport error type
#[derive(Debug)]
pub enum TransportError {
    /// Validation error (origin, headers, protocol version, etc.)
    Validation(String),

    /// OAuth authentication error (invalid/missing token)
    /// Includes WWW-Authenticate header value
    Unauthorized {
        message: String,
        www_authenticate: Option<String>,
    },

    /// OAuth authorization error (valid token, insufficient permissions)
    /// TODO: Implement authorization check in transport layer
    /// MCP spec requires 403 Forbidden for authorization failures (mpc-auth.md:286)
    /// Currently only doing authentication (decode), not authorization (authorize)
    /// See: .agent/oauth/mpc-auth.md for spec requirements
    #[allow(dead_code)]
    Forbidden(String),

    /// I/O error from server-io operations
    Io(IoError),

    /// Protocol-level error (MCP protocol violations)
    Protocol(String),

    /// Session management error
    Session(String),

    /// Internal error (should not happen in normal operation)
    Internal(String),
}

impl TransportError {
    /// Create a validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    /// Create an unauthorized error (401) with optional WWW-Authenticate header
    /// TODO: Currently unused - use unauthorized_with_challenge instead
    /// Kept for completeness when authorization is implemented
    #[allow(dead_code)]
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::Unauthorized {
            message: msg.into(),
            www_authenticate: None,
        }
    }

    /// Create an unauthorized error with WWW-Authenticate header
    pub fn unauthorized_with_challenge(msg: impl Into<String>, www_authenticate: String) -> Self {
        Self::Unauthorized {
            message: msg.into(),
            www_authenticate: Some(www_authenticate),
        }
    }

    /// Create a forbidden error (403)
    /// TODO: Will be used when server_auth::authorize() is called in transport
    /// Need to decide on authorization architecture first:
    /// - Per-middleware policies vs single global policy
    /// - Environment variable naming (POLICY vs POLICY_TOOLS, etc.)
    #[allow(dead_code)]
    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::Forbidden(msg.into())
    }

    /// Create a protocol error
    pub fn protocol(msg: impl Into<String>) -> Self {
        Self::Protocol(msg.into())
    }

    /// Create a session error
    pub fn session(msg: impl Into<String>) -> Self {
        Self::Session(msg.into())
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Get HTTP status code for this error
    pub fn http_status_code(&self) -> u16 {
        match self {
            Self::Validation(_) => 400,
            Self::Unauthorized { .. } => 401,
            Self::Forbidden(_) => 403,
            Self::Protocol(_) => 400,
            Self::Session(msg) if msg.contains("not found") => 404,
            Self::Session(msg) if msg.contains("terminated") => 404,
            Self::Session(msg) if msg.contains("required") => 400,
            Self::Session(_) => 500,
            Self::Io(_) => 500,
            Self::Internal(_) => 500,
        }
    }

    /// Get WWW-Authenticate header value if present
    pub fn www_authenticate_header(&self) -> Option<&str> {
        match self {
            Self::Unauthorized {
                www_authenticate: Some(header),
                ..
            } => Some(header),
            _ => None,
        }
    }

    /// Get error message
    pub fn message(&self) -> String {
        match self {
            Self::Validation(msg) => msg.clone(),
            Self::Unauthorized { message, .. } => message.clone(),
            Self::Forbidden(msg) => msg.clone(),
            Self::Protocol(msg) => msg.clone(),
            Self::Session(msg) => msg.clone(),
            Self::Io(e) => format!("I/O error: {:?}", e),
            Self::Internal(msg) => msg.clone(),
        }
    }
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for TransportError {}

// Automatic conversions from other error types

impl From<IoError> for TransportError {
    fn from(e: IoError) -> Self {
        Self::Io(e)
    }
}

impl From<ErrorCode> for TransportError {
    fn from(e: ErrorCode) -> Self {
        Self::Protocol(format!("{:?}", e))
    }
}

impl From<String> for TransportError {
    fn from(s: String) -> Self {
        Self::Internal(s)
    }
}

impl From<&str> for TransportError {
    fn from(s: &str) -> Self {
        Self::Internal(s.to_string())
    }
}
