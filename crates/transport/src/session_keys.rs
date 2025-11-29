//! Session storage key constants
//!
//! Centralized constants for session key-value storage to prevent typos
//! and maintain consistency across the codebase.

/// JWT subject claim stored in session
pub const JWT_SUBJECT: &str = "jwt:sub";

/// JWT issuer claim stored in session
pub const JWT_ISSUER: &str = "jwt:iss";

/// JWT scopes stored as comma-separated list
pub const JWT_SCOPES: &str = "jwt:scopes";

/// JWT audiences stored as comma-separated list
pub const JWT_AUDIENCES: &str = "jwt:audiences";

/// JWT expiration timestamp (Unix epoch seconds)
pub const JWT_EXPIRATION: &str = "jwt:exp";

/// JWT issued-at timestamp (Unix epoch seconds)
pub const JWT_ISSUED_AT: &str = "jwt:iat";
