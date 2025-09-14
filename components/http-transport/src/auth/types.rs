use serde::{Deserialize, Serialize};

// Use WIT AuthContext directly from bindings and re-export it
pub use crate::bindings::wasmcp::transport::tools::AuthContext;

// Internal auth types for transport implementation

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub token: String,
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub expected_issuer: String,
    pub expected_audiences: Vec<String>,
    pub expected_subject: Option<String>,
    pub jwks_uri: String,
    pub policy: Option<String>,
    pub policy_data: Option<String>,
    pub pass_jwt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthError {
    pub status: u16,
    pub error_code: String,
    pub description: String,
    pub www_authenticate: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AuthResponse {
    Authorized(AuthContext),
    Unauthorized(AuthError),
}

// JWT types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtRequest {
    pub token: String,
    pub expected_issuer: String,
    pub expected_audiences: Vec<String>,
    pub expected_subject: Option<String>,
    pub jwks_uri: String,
    pub jwks_json: Option<String>,
    pub clock_skew: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub iss: String,
    pub aud: Option<Vec<String>>,
    pub exp: Option<u64>,
    pub iat: Option<u64>,
    pub nbf: Option<u64>,
    pub jti: Option<String>,
    pub scopes: Vec<String>,
    pub client_id: Option<String>,
    pub additional_claims: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JwtError {
    Expired,
    InvalidSignature,
    InvalidIssuer,
    InvalidAudience,
    Malformed,
    NotYetValid,
    MissingClaim,
    JwksError,
    UnknownKid,
    Other,
}

#[derive(Debug, Clone)]
pub enum JwtResult {
    Valid(JwtClaims),
    Invalid(JwtError),
}

// Policy types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRequest {
    pub policy: String,
    pub data: Option<String>,
    pub input: String,
    pub query: Option<String>,
}

#[derive(Debug, Clone)]
pub enum PolicyResult {
    Allow,
    Deny(String),
    Error(String),
}

// OAuth discovery types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetadata {
    pub resource_url: String,
    pub authorization_servers: Vec<String>,
    pub scopes_supported: Option<Vec<String>>,
    pub bearer_methods_supported: Option<Vec<String>>,
    pub resource_documentation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<String>,
    pub scopes_supported: Option<Vec<String>>,
    pub token_endpoint_auth_methods_supported: Option<Vec<String>>,
    pub service_documentation: Option<String>,
    pub registration_endpoint: Option<String>,
}
