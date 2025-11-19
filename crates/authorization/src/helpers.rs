//! OAuth claim helper functions for MCP sessions
//!
//! These helpers allow tools to work with JWT claims from MessageContext.identity
//! without needing to import separate OAuth packages.
//!
//! NOTE: Claims arrive as structured jwt-claims in MessageContext.identity.claims
//! No parsing is needed - use these helpers directly on the structured claims!

use crate::bindings::wasmcp::auth::types::JwtClaims;
use jsonwebtoken::get_current_timestamp;

/// Convert structured JWT claims to flat format for storage/serialization
///
/// Useful when storing claims in session KV or passing to external systems.
/// Most tools won't need this - claims are already structured in MessageContext!
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

/// Get a custom claim value by key
///
/// Returns value if claim exists in custom-claims or standard fields.
pub fn get_claim(claims: &JwtClaims, key: &str) -> Option<String> {
    // Check standard fields first
    match key {
        "sub" => Some(claims.subject.clone()),
        "iss" => claims.issuer.clone(),
        "aud" => {
            if !claims.audience.is_empty() {
                Some(claims.audience.join(","))
            } else {
                None
            }
        }
        "exp" => claims.expiration.map(|e| e.to_string()),
        "iat" => claims.issued_at.map(|i| i.to_string()),
        "nbf" => claims.not_before.map(|n| n.to_string()),
        "jti" => claims.jwt_id.clone(),
        "scope" | "scp" => {
            if !claims.scopes.is_empty() {
                Some(claims.scopes.join(" "))
            } else {
                None
            }
        }
        _ => {
            // Check custom claims
            claims
                .custom_claims
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
        }
    }
}

/// Check if claims contain specific scope
///
/// OAuth scopes are space-separated in the 'scope' or 'scp' claim.
/// This checks if the specified scope exists in the token.
pub fn has_scope(claims: &JwtClaims, scope: &str) -> bool {
    claims.scopes.iter().any(|s| s == scope)
}

/// Check if claims contain any of the specified scopes
pub fn has_any_scope(claims: &JwtClaims, scopes: &[String]) -> bool {
    scopes.iter().any(|scope| has_scope(claims, scope))
}

/// Check if claims contain all of the specified scopes
pub fn has_all_scopes(claims: &JwtClaims, scopes: &[String]) -> bool {
    scopes.iter().all(|scope| has_scope(claims, scope))
}

/// Validate audience claim
///
/// Checks if any of the token's audiences match the expected value.
/// Critical for preventing confused deputy attacks.
pub fn has_audience(claims: &JwtClaims, audience: &str) -> bool {
    claims.audience.iter().any(|aud| aud == audience)
}

/// Check if token is expired
///
/// Compares exp claim against current time with optional clock skew.
pub fn is_expired(claims: &JwtClaims, clock_skew_seconds: Option<u64>) -> bool {
    if let Some(exp) = claims.expiration {
        let now = get_current_timestamp();
        let skew = clock_skew_seconds.unwrap_or(0);
        now >= exp.saturating_sub(skew)
    } else {
        // No expiration means token doesn't expire
        false
    }
}

/// Check if token time is valid (nbf <= now < exp)
pub fn is_valid_time(claims: &JwtClaims, clock_skew_seconds: Option<u64>) -> bool {
    let now = get_current_timestamp();
    let skew = clock_skew_seconds.unwrap_or(0);

    // Check not-before
    if let Some(nbf) = claims.not_before
        && now < nbf.saturating_sub(skew)
    {
        return false;
    }

    // Check expiration
    if let Some(exp) = claims.expiration
        && now >= exp.saturating_sub(skew)
    {
        return false;
    }

    true
}

/// Get subject (user ID)
pub fn get_subject(claims: &JwtClaims) -> String {
    claims.subject.clone()
}

/// Get issuer
pub fn get_issuer(claims: &JwtClaims) -> Option<String> {
    claims.issuer.clone()
}

/// Get all scopes as list
pub fn get_scopes(claims: &JwtClaims) -> Vec<String> {
    claims.scopes.clone()
}

/// Get all audiences as list
pub fn get_audiences(claims: &JwtClaims) -> Vec<String> {
    claims.audience.clone()
}
