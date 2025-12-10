//! JWT token decoding and display

use anyhow::{Context, Result};
use jsonwebtoken::decode_header;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::format_timestamp;

/// Claims structure for decoding (flexible)
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    #[serde(default)]
    sub: String,
    #[serde(default)]
    iss: String,
    #[serde(default)]
    aud: Option<AudienceValue>,
    exp: u64,
    iat: u64,
    #[serde(default)]
    nbf: Option<u64>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(flatten)]
    custom: HashMap<String, Value>,
}

/// Audience can be string or array
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum AudienceValue {
    Single(String),
    Multiple(Vec<String>),
}

/// Calculate validation status
fn get_validation_status(exp: u64, nbf: Option<u64>) -> String {
    let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => {
            // System clock is before Unix epoch - extremely rare edge case
            return "INVALID (system clock error)".to_string();
        }
    };

    // Check not-before
    if let Some(nbf_val) = nbf
        && now < nbf_val
    {
        return "NOT YET VALID (nbf not reached)".to_string();
    }

    // Check expiration
    if exp <= now {
        return "EXPIRED".to_string();
    }

    // Calculate time until expiration
    let remaining = exp - now;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;

    if hours > 0 {
        format!(
            "VALID (expires in {} hour{} {} minute{})",
            hours,
            if hours == 1 { "" } else { "s" },
            minutes,
            if minutes == 1 { "" } else { "s" }
        )
    } else if minutes > 0 {
        format!(
            "VALID (expires in {} minute{})",
            minutes,
            if minutes == 1 { "" } else { "s" }
        )
    } else {
        format!(
            "VALID (expires in {} second{})",
            remaining,
            if remaining == 1 { "" } else { "s" }
        )
    }
}

/// Decode a JWT token (accepts token string or saved name)
pub fn decode_token(token_or_name: &str) -> Result<()> {
    // Check if it's a saved token name or raw JWT
    let token = if token_or_name.starts_with("eyJ") {
        // Looks like a JWT token
        token_or_name.to_string()
    } else {
        // Try to load as saved token
        super::storage::load_stored_token(token_or_name)?
    };

    // Decode header (no validation needed)
    let header = decode_header(&token).context("Failed to decode JWT header")?;

    println!("Header:");
    println!("  alg: {:?}", header.alg);
    println!("  typ: {}", header.typ.unwrap_or_else(|| "JWT".to_string()));
    if let Some(kid) = header.kid {
        println!("  kid: {}", kid);
    }
    println!();

    // Decode claims without validation (for display purposes)
    // We manually decode the payload since we're just displaying it
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        anyhow::bail!(
            "Invalid JWT format - expected 3 parts separated by dots, got {} parts",
            parts.len()
        );
    }

    // Decode the payload (second part)
    use base64::{Engine as _, engine::general_purpose};
    let payload_bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .context("Failed to decode JWT payload")?;

    let claims: Claims =
        serde_json::from_slice(&payload_bytes).context("Failed to parse JWT claims")?;

    println!("Claims:");
    println!("  sub: {}", claims.sub);
    println!("  iss: {}", claims.iss);

    if let Some(aud) = claims.aud {
        match aud {
            AudienceValue::Single(s) => println!("  aud: {}", s),
            AudienceValue::Multiple(v) => println!("  aud: {}", v.join(", ")),
        }
    }

    println!("  iat: {} ({})", claims.iat, format_timestamp(claims.iat));
    println!("  exp: {} ({})", claims.exp, format_timestamp(claims.exp));

    if let Some(nbf) = claims.nbf {
        println!("  nbf: {} ({})", nbf, format_timestamp(nbf));
    }

    if let Some(scope) = &claims.scope {
        println!("  scope: {}", scope);
    }

    // Display custom claims
    let standard_fields = ["sub", "iss", "aud", "exp", "iat", "nbf", "scope"];
    let mut custom_claims: Vec<_> = claims
        .custom
        .iter()
        .filter(|(k, _)| !standard_fields.contains(&k.as_str()))
        .collect();

    if !custom_claims.is_empty() {
        println!();
        println!("Custom Claims:");
        custom_claims.sort_by_key(|(k, _)| *k);

        for (key, value) in custom_claims {
            let value_str = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Array(_) => {
                    serde_json::to_string(&value).unwrap_or_else(|_| "???".to_string())
                }
                Value::Object(_) => {
                    serde_json::to_string(&value).unwrap_or_else(|_| "???".to_string())
                }
                Value::Null => "null".to_string(),
            };
            println!("  {}: {}", key, value_str);
        }
    }

    println!();
    println!("Status: {}", get_validation_status(claims.exp, claims.nbf));

    Ok(())
}
