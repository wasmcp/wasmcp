//! Helper functions for JWT claims

use crate::bindings::wasmcp::oauth::types::JwtClaims;
use std::time::{SystemTime, UNIX_EPOCH};

/// Get a custom claim value
pub fn get_claim(claims: &JwtClaims, key: &str) -> Option<String> {
    claims
        .custom_claims
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.clone())
}

/// Check if a specific scope is present
pub fn has_scope(claims: &JwtClaims, scope: &str) -> bool {
    claims.scopes.iter().any(|s| s == scope)
}

/// Check if ANY of the provided scopes are present
pub fn has_any_scope(claims: &JwtClaims, scopes: &[String]) -> bool {
    scopes.iter().any(|scope| has_scope(claims, scope))
}

/// Check if ALL of the provided scopes are present
pub fn has_all_scopes(claims: &JwtClaims, scopes: &[String]) -> bool {
    scopes.iter().all(|scope| has_scope(claims, scope))
}

/// Check if a specific audience is present
pub fn has_audience(claims: &JwtClaims, audience: &str) -> bool {
    claims.audience.iter().any(|aud| aud == audience)
}

/// Check if the token is expired
pub fn is_expired(claims: &JwtClaims) -> bool {
    if let Some(exp) = claims.expiration {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= exp
    } else {
        // No expiration means token doesn't expire
        false
    }
}

/// Check if the token is valid based on time claims (nbf, exp)
pub fn is_valid_time(claims: &JwtClaims) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Check not-before
    if let Some(nbf) = claims.not_before {
        if now < nbf {
            return false;
        }
    }

    // Check expiration
    if let Some(exp) = claims.expiration {
        if now >= exp {
            return false;
        }
    }

    true
}

/// Get the subject (user ID)
pub fn get_subject(claims: &JwtClaims) -> String {
    claims.subject.clone()
}

/// Get the issuer
pub fn get_issuer(claims: &JwtClaims) -> Option<String> {
    claims.issuer.clone()
}

/// Get all scopes
pub fn get_scopes(claims: &JwtClaims) -> Vec<String> {
    claims.scopes.clone()
}
