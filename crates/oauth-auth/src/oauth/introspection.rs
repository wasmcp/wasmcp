//! OAuth 2.0 Token Introspection (RFC 7662)

use crate::bindings::exports::wasmcp::oauth::errors::OauthError;
use crate::bindings::exports::wasmcp::oauth::introspection::{
    IntrospectionRequest, IntrospectionResponse,
};
use crate::bindings::wasmcp::oauth::types::JwtClaims;

/// Introspect an opaque token with authorization server
/// TODO: Implement HTTP request to AS introspection endpoint
pub fn introspect_token(
    _introspection_endpoint: &str,
    _request: &IntrospectionRequest,
    _client_credentials: &(String, String),
) -> Result<IntrospectionResponse, OauthError> {
    // This would make an HTTP POST to the AS introspection endpoint
    // For now, return error indicating not implemented
    use crate::bindings::exports::wasmcp::oauth::errors::ErrorCode;
    Err(OauthError {
        error: ErrorCode::ServerError,
        error_description: Some("Token introspection not yet implemented".to_string()),
        error_uri: None,
    })
}

/// Convert introspection response to jwt-claims
pub fn to_jwt_claims(response: &IntrospectionResponse) -> Option<JwtClaims> {
    // If token is not active, return None
    if !response.active {
        return None;
    }

    // Build jwt-claims from introspection response
    Some(JwtClaims {
        subject: response.sub.clone().unwrap_or_default(),
        issuer: response.iss.clone(),
        audience: response.aud.clone().unwrap_or_default(),
        expiration: response.exp,
        issued_at: response.iat,
        not_before: response.nbf,
        jwt_id: response.jti.clone(),
        scopes: response.scope.clone().unwrap_or_default(),
        confirmation: None, // Introspection doesn't include confirmation
        custom_claims: response.additional_claims.clone(),
    })
}
