//! JWT token verification

use crate::bindings::wasmcp::auth::types::JwtClaims;
use crate::config::JwtProvider;
use crate::error::{AuthError, Result};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    /// Subject
    sub: String,

    /// Issuer
    iss: String,

    /// Audience (can be string or array)
    #[serde(skip_serializing_if = "Option::is_none")]
    aud: Option<AudienceValue>,

    /// Expiration time
    exp: i64,

    /// Issued at
    iat: i64,

    /// Not before
    #[serde(skip_serializing_if = "Option::is_none")]
    nbf: Option<i64>,

    /// OAuth2 scope claim
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,

    /// Microsoft-style scope claim (can be string or array)
    #[serde(skip_serializing_if = "Option::is_none")]
    scp: Option<ScopeValue>,

    /// Additional claims (captures all other claims)
    #[serde(flatten)]
    additional: serde_json::Map<String, serde_json::Value>,
}

/// Audience value (can be string or array)
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum AudienceValue {
    Single(String),
    Multiple(Vec<String>),
}

/// Scope value (can be string or array)
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum ScopeValue {
    String(String),
    List(Vec<String>),
}

/// Verify a JWT token using the provided configuration
/// Returns structured JwtClaims directly (WIT type)
pub fn verify(token: &str, provider: &JwtProvider) -> Result<JwtClaims> {
    // Decode header to get algorithm and key ID
    let header = decode_header(token)?;

    // Get decoding key - JWKS takes precedence over static key
    let decoding_key = if let Some(ref jwks_uri) = provider.jwks_uri {
        // Dynamic key fetching via JWKS
        let jwks = crate::jwks::fetch_jwks(jwks_uri)?;

        // Find key matching the KID from token header
        crate::jwks::find_key(&jwks, header.kid.as_deref())?
    } else if let Some(ref public_key) = provider.public_key {
        // Static public key fallback
        DecodingKey::from_rsa_pem(public_key.as_bytes())
            .map_err(|e| AuthError::Configuration(format!("Invalid public key: {e}")))?
    } else {
        return Err(AuthError::Configuration(
            "No public key or JWKS URI configured".to_string(),
        ));
    };

    // Set up validation using configured algorithm (defaults to RS256)
    let algorithm = match provider.algorithm.as_deref().unwrap_or("RS256") {
        "HS256" => Algorithm::HS256,
        "HS384" => Algorithm::HS384,
        "HS512" => Algorithm::HS512,
        "RS256" => Algorithm::RS256,
        "RS384" => Algorithm::RS384,
        "RS512" => Algorithm::RS512,
        "ES256" => Algorithm::ES256,
        "ES384" => Algorithm::ES384,
        "PS256" => Algorithm::PS256,
        "PS384" => Algorithm::PS384,
        "PS512" => Algorithm::PS512,
        alg => {
            return Err(AuthError::Configuration(format!(
                "Unsupported algorithm: {alg}"
            )));
        }
    };
    let mut validation = Validation::new(algorithm);

    // Set issuer validation (only if configured)
    if !provider.issuer.is_empty() {
        validation.set_issuer(&[&provider.issuer]);
    }

    // Set audience validation (only if configured)
    if !provider.audience.is_empty() {
        validation.set_audience(&provider.audience);
    } else {
        validation.validate_aud = false;
    }

    // Enable nbf (not before) validation if present in token
    validation.validate_nbf = true;

    // Add leeway for clock skew tolerance (60 seconds)
    validation.leeway = 60;

    // Set required claims
    validation.set_required_spec_claims(&["exp", "sub", "iss"]);

    // Decode and validate token
    let token_data =
        decode::<Claims>(token, &decoding_key, &validation).map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::InvalidToken => {
                AuthError::InvalidToken(format!("Invalid token: {}", e))
            }
            jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                AuthError::InvalidToken("Invalid signature".to_string())
            }
            jsonwebtoken::errors::ErrorKind::InvalidEcdsaKey => {
                AuthError::InvalidToken("Invalid ECDSA key".to_string())
            }
            jsonwebtoken::errors::ErrorKind::InvalidRsaKey(_) => {
                AuthError::InvalidToken("Invalid RSA key".to_string())
            }
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                AuthError::InvalidToken("Token expired".to_string())
            }
            jsonwebtoken::errors::ErrorKind::InvalidIssuer => {
                AuthError::InvalidToken("Invalid issuer".to_string())
            }
            jsonwebtoken::errors::ErrorKind::InvalidAudience => {
                AuthError::InvalidToken("Invalid audience".to_string())
            }
            jsonwebtoken::errors::ErrorKind::InvalidSubject => {
                AuthError::InvalidToken("Invalid subject".to_string())
            }
            jsonwebtoken::errors::ErrorKind::ImmatureSignature => {
                AuthError::InvalidToken("Token not yet valid (nbf)".to_string())
            }
            jsonwebtoken::errors::ErrorKind::InvalidAlgorithm => {
                AuthError::InvalidToken("Invalid algorithm".to_string())
            }
            _ => AuthError::InvalidToken(format!("Token validation error: {}", e)),
        })?;
    let claims = token_data.claims;

    // Extract scopes
    let scopes = extract_scopes(&claims);

    // Check required scopes
    if let Some(required_scopes) = &provider.required_scopes {
        use std::collections::HashSet;

        let token_scopes: HashSet<String> = scopes.iter().cloned().collect();
        let required_set: HashSet<String> = required_scopes.iter().cloned().collect();

        if !required_set.is_subset(&token_scopes) {
            let missing_scopes: Vec<String> =
                required_set.difference(&token_scopes).cloned().collect();
            return Err(AuthError::Unauthorized(format!(
                "Token missing required scopes: {missing_scopes:?}"
            )));
        }
    }

    // Extract audience
    let audience = match claims.aud {
        Some(AudienceValue::Single(s)) => vec![s],
        Some(AudienceValue::Multiple(v)) => v,
        None => Vec::new(),
    };

    // Extract timestamps (convert i64 to u64)
    let expiration = Some(claims.exp as u64);
    let issued_at = Some(claims.iat as u64);
    let not_before = claims.nbf.map(|nbf| nbf as u64);

    // Extract JWT ID if present
    let jwt_id = claims
        .additional
        .get("jti")
        .and_then(|v| v.as_str().map(String::from));

    // Build custom claims (exclude standard fields)
    let standard_fields = [
        "sub", "iss", "aud", "exp", "iat", "nbf", "jti", "scope", "scp", "cnf",
    ];
    let custom_claims: Vec<(String, String)> = claims
        .additional
        .into_iter()
        .filter(|(k, _)| !standard_fields.contains(&k.as_str()))
        .map(|(k, v)| {
            let value_str = match v {
                serde_json::Value::String(s) => s,
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Null => "null".to_string(),
                _ => serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()),
            };
            (k, value_str)
        })
        .collect();

    // Build JwtClaims directly - NO intermediate type!
    Ok(JwtClaims {
        subject: claims.sub,
        issuer: Some(claims.iss),
        audience,
        expiration,
        issued_at,
        not_before,
        jwt_id,
        scopes,
        confirmation: None, // TODO: Extract cnf claim if present
        custom_claims,
    })
}

/// Extract scopes from claims
fn extract_scopes(claims: &Claims) -> Vec<String> {
    // OAuth2 'scope' claim takes precedence
    if let Some(scope) = &claims.scope {
        return scope.split_whitespace().map(String::from).collect();
    }

    // Fall back to Microsoft 'scp' claim
    if let Some(scp) = &claims.scp {
        return match scp {
            ScopeValue::String(s) => s.split_whitespace().map(String::from).collect(),
            ScopeValue::List(list) => list.clone(),
        };
    }

    // No scopes
    Vec::new()
}
