//! JWT token verification

use crate::config::JwtProvider;
use crate::error::{AuthError, Result};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Token information extracted from a verified JWT
#[derive(Debug, Clone)]
pub struct TokenInfo {
    /// Subject (user ID)
    pub sub: String,

    /// Issuer
    pub iss: String,

    /// Scopes
    pub scopes: Vec<String>,

    /// All claims from the token (for authorization and forwarding)
    pub claims: HashMap<String, serde_json::Value>,
}

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
pub fn verify(token: &str, provider: &JwtProvider) -> Result<TokenInfo> {
    eprintln!("[oauth-auth:jwt] Starting JWT verification");
    eprintln!("[oauth-auth:jwt] Token length: {}", token.len());

    // Decode header to get algorithm and key ID
    let header = decode_header(token).map_err(|e| {
        eprintln!("[oauth-auth:jwt] ✗ Failed to decode JWT header: {:?}", e);
        e
    })?;

    eprintln!(
        "[oauth-auth:jwt] ✓ Decoded JWT header - algorithm: {:?}, kid: {:?}",
        header.alg, header.kid
    );

    // Get decoding key - JWKS takes precedence over static key
    let decoding_key = if let Some(ref jwks_uri) = provider.jwks_uri {
        eprintln!("[oauth-auth:jwt] Using JWKS URI: {}", jwks_uri);

        // Dynamic key fetching via JWKS
        let jwks = crate::jwks::fetch_jwks(jwks_uri).map_err(|e| {
            eprintln!("[oauth-auth:jwt] ✗ Failed to fetch JWKS: {:?}", e);
            e
        })?;

        eprintln!(
            "[oauth-auth:jwt] ✓ Fetched JWKS with {} keys",
            jwks.keys.len()
        );

        // Find key matching the KID from token header
        crate::jwks::find_key(&jwks, header.kid.as_deref()).map_err(|e| {
            eprintln!("[oauth-auth:jwt] ✗ Failed to find key in JWKS: {:?}", e);
            e
        })?
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

    eprintln!("[oauth-auth:jwt] Validation config:");
    eprintln!("[oauth-auth:jwt]   Algorithm: {:?}", algorithm);

    // Set issuer validation (only if configured)
    if !provider.issuer.is_empty() {
        eprintln!("[oauth-auth:jwt]   Expected issuer: {}", provider.issuer);
        validation.set_issuer(&[&provider.issuer]);
    } else {
        eprintln!("[oauth-auth:jwt]   Issuer validation: disabled");
    }

    // Set audience validation (only if configured)
    if !provider.audience.is_empty() {
        eprintln!(
            "[oauth-auth:jwt]   Expected audience(s): {:?}",
            provider.audience
        );
        validation.set_audience(&provider.audience);
    } else {
        eprintln!("[oauth-auth:jwt]   Audience validation: disabled (for dynamic registration)");
        validation.validate_aud = false;
    }

    // Enable nbf (not before) validation if present in token
    validation.validate_nbf = true;

    // Add leeway for clock skew tolerance (60 seconds)
    validation.leeway = 60;

    // Set required claims
    validation.set_required_spec_claims(&["exp", "sub", "iss"]);

    eprintln!("[oauth-auth:jwt] Decoding and validating token...");

    // First decode WITHOUT validation to see the actual claims
    let mut temp_validation = Validation::new(algorithm);
    temp_validation.insecure_disable_signature_validation();
    temp_validation.validate_aud = false;
    temp_validation.validate_exp = false;
    temp_validation.required_spec_claims.clear();

    if let Ok(unvalidated) = decode::<Claims>(token, &DecodingKey::from_secret(&[]), &temp_validation) {
        eprintln!("[oauth-auth:jwt] Token claims (unvalidated):");
        eprintln!("[oauth-auth:jwt]   Actual audience in token: {:?}", unvalidated.claims.aud);
        eprintln!("[oauth-auth:jwt]   Actual issuer in token: {}", unvalidated.claims.iss);
    }

    // Now decode and validate token properly
    let token_data = decode::<Claims>(token, &decoding_key, &validation).map_err(|e| {
        eprintln!("[oauth-auth:jwt] ✗ Token validation failed: {:?}", e);
        match e.kind() {
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
        }
    })?;
    let claims = token_data.claims;

    eprintln!("[oauth-auth:jwt] ✓ Token successfully decoded and validated");
    eprintln!("[oauth-auth:jwt] Token claims:");
    eprintln!("[oauth-auth:jwt]   sub: {}", claims.sub);
    eprintln!("[oauth-auth:jwt]   iss: {}", claims.iss);
    eprintln!("[oauth-auth:jwt]   aud: {:?}", claims.aud);
    eprintln!("[oauth-auth:jwt]   exp: {}", claims.exp);
    eprintln!("[oauth-auth:jwt]   iat: {}", claims.iat);

    // Extract scopes
    let scopes = extract_scopes(&claims);
    eprintln!("[oauth-auth:jwt]   scopes: {:?}", scopes);

    // Check required scopes
    if let Some(required_scopes) = &provider.required_scopes {
        eprintln!(
            "[oauth-auth:jwt] Checking required scopes: {:?}",
            required_scopes
        );
        use std::collections::HashSet;

        let token_scopes: HashSet<String> = scopes.iter().cloned().collect();
        let required_set: HashSet<String> = required_scopes.iter().cloned().collect();

        if !required_set.is_subset(&token_scopes) {
            let missing_scopes: Vec<String> =
                required_set.difference(&token_scopes).cloned().collect();
            eprintln!(
                "[oauth-auth:jwt] ✗ Missing required scopes: {:?}",
                missing_scopes
            );
            return Err(AuthError::Unauthorized(format!(
                "Token missing required scopes: {missing_scopes:?}"
            )));
        }
        eprintln!("[oauth-auth:jwt] ✓ All required scopes present");
    }

    // Build complete claims map
    let mut all_claims = HashMap::new();
    all_claims.insert(
        "sub".to_string(),
        serde_json::Value::String(claims.sub.clone()),
    );
    all_claims.insert(
        "iss".to_string(),
        serde_json::Value::String(claims.iss.clone()),
    );
    if let Some(aud) = claims.aud {
        all_claims.insert(
            "aud".to_string(),
            match aud {
                AudienceValue::Single(s) => serde_json::Value::String(s),
                AudienceValue::Multiple(v) => serde_json::json!(v),
            },
        );
    }
    all_claims.insert("exp".to_string(), serde_json::json!(claims.exp));
    all_claims.insert("iat".to_string(), serde_json::json!(claims.iat));
    if let Some(nbf) = claims.nbf {
        all_claims.insert("nbf".to_string(), serde_json::json!(nbf));
    }
    if let Some(scope) = claims.scope {
        all_claims.insert("scope".to_string(), serde_json::Value::String(scope));
    }
    if let Some(scp) = claims.scp {
        all_claims.insert(
            "scp".to_string(),
            match scp {
                ScopeValue::String(s) => serde_json::Value::String(s),
                ScopeValue::List(v) => serde_json::json!(v),
            },
        );
    }
    // Add all additional claims
    for (key, value) in claims.additional {
        all_claims.insert(key, value);
    }

    eprintln!(
        "[oauth-auth:jwt] ✓ JWT verification complete - user: {}",
        claims.sub
    );

    Ok(TokenInfo {
        sub: claims.sub,
        iss: claims.iss,
        scopes,
        claims: all_claims,
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
