//! Session claims bridge for MCP

use crate::bindings::exports::wasmcp::oauth::errors::{ErrorCode, OauthError};
use crate::bindings::wasmcp::oauth::types::JwtClaims;

/// Parse flat MCP claims to structured JWT claims
pub fn parse_claims(flat_claims: Vec<(String, String)>) -> Result<JwtClaims, OauthError> {
    let mut claims = JwtClaims {
        subject: String::new(),
        issuer: None,
        audience: Vec::new(),
        expiration: None,
        issued_at: None,
        not_before: None,
        jwt_id: None,
        scopes: Vec::new(),
        confirmation: None,
        custom_claims: Vec::new(),
    };

    for (key, value) in flat_claims {
        match key.as_str() {
            "sub" => claims.subject = value,
            "iss" => claims.issuer = Some(value),
            "aud" => {
                // Parse as comma-separated array
                claims.audience = value.split(',').map(|s| s.trim().to_string()).collect();
            }
            "exp" => claims.expiration = value.parse().ok(),
            "iat" => claims.issued_at = value.parse().ok(),
            "nbf" => claims.not_before = value.parse().ok(),
            "jti" => claims.jwt_id = Some(value),
            "scope" | "scp" => {
                // Split whitespace-separated scopes
                claims.scopes = value.split_whitespace().map(String::from).collect();
            }
            _ => {
                // Everything else goes to custom claims
                claims.custom_claims.push((key, value));
            }
        }
    }

    // Validate required fields
    if claims.subject.is_empty() {
        return Err(OauthError {
            error: ErrorCode::InvalidToken,
            error_description: Some("Missing required 'sub' claim".to_string()),
            error_uri: None,
        });
    }

    Ok(claims)
}

/// Flatten structured JWT claims to MCP format
pub fn flatten_claims(claims: &JwtClaims) -> Vec<(String, String)> {
    let mut flat = vec![("sub".to_string(), claims.subject.clone())];

    if let Some(iss) = &claims.issuer {
        flat.push(("iss".to_string(), iss.clone()));
    }

    if !claims.audience.is_empty() {
        flat.push(("aud".to_string(), claims.audience.join(",")));
    }

    if let Some(exp) = claims.expiration {
        flat.push(("exp".to_string(), exp.to_string()));
    }

    if let Some(iat) = claims.issued_at {
        flat.push(("iat".to_string(), iat.to_string()));
    }

    if let Some(nbf) = claims.not_before {
        flat.push(("nbf".to_string(), nbf.to_string()));
    }

    if let Some(jti) = &claims.jwt_id {
        flat.push(("jti".to_string(), jti.clone()));
    }

    if !claims.scopes.is_empty() {
        flat.push(("scope".to_string(), claims.scopes.join(" ")));
    }

    // Add custom claims
    flat.extend(claims.custom_claims.clone());

    flat
}

/// Check if a session has a specific claim
/// TODO: Implement session storage lookup
pub fn has_claim(_session_id: &str, _claim_key: &str) -> Option<String> {
    // This would need to integrate with session storage
    None
}

/// Check if a session has a specific scope
/// TODO: Implement session storage lookup
pub fn has_session_scope(_session_id: &str, _scope: &str) -> bool {
    // This would need to integrate with session storage
    false
}

/// Get full structured claims from a session
/// TODO: Implement session storage lookup
pub fn get_session_claims(_session_id: &str) -> Option<JwtClaims> {
    // This would need to integrate with session storage
    None
}
