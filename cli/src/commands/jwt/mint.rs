//! JWT token minting for testing

use anyhow::{Context, Result};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{format_timestamp, get_jwt_test_dir};

/// JWT signing algorithm (RS256 for RSA-2048 keys)
const JWT_ALGORITHM: Algorithm = Algorithm::RS256;

/// JWT Claims structure for minting
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject
    pub sub: String,

    /// Issuer
    pub iss: String,

    /// Audience (optional)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aud: Vec<String>,

    /// Expiration time
    pub exp: u64,

    /// Issued at
    pub iat: u64,

    /// Not before (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nbf: Option<u64>,

    /// Scope claim (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,

    /// Additional custom claims
    #[serde(flatten)]
    pub custom: HashMap<String, Value>,
}

/// Parse custom claims from key=value strings
fn parse_custom_claims(claim_strings: Vec<String>) -> Result<HashMap<String, Value>> {
    let mut claims = HashMap::new();

    for claim_str in claim_strings {
        let parts: Vec<&str> = claim_str.splitn(2, '=').collect();
        if parts.len() != 2 {
            anyhow::bail!(
                "Invalid claim format '{}'.\nExpected format: key=value",
                claim_str
            );
        }

        let key = parts[0].to_string();
        let value = parts[1];

        // Try to parse as JSON value (number, boolean, string, etc.)
        let json_value = if let Ok(num) = value.parse::<i64>() {
            Value::Number(num.into())
        } else if let Ok(b) = value.parse::<bool>() {
            Value::Bool(b)
        } else {
            Value::String(value.to_string())
        };

        claims.insert(key, json_value);
    }

    Ok(claims)
}

/// Calculate duration until expiration
fn time_until_expiration(exp: u64) -> String {
    let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => {
            // System clock is before Unix epoch - extremely rare edge case
            return "INVALID (system clock error)".to_string();
        }
    };

    if exp <= now {
        "EXPIRED".to_string()
    } else {
        let remaining = exp - now;
        let hours = remaining / 3600;
        let minutes = (remaining % 3600) / 60;

        if hours > 0 {
            format!(
                "{} hour{} {} minute{}",
                hours,
                if hours == 1 { "" } else { "s" },
                minutes,
                if minutes == 1 { "" } else { "s" }
            )
        } else {
            format!("{} minute{}", minutes, if minutes == 1 { "" } else { "s" })
        }
    }
}

/// Configuration for minting a test JWT token
pub struct MintTokenConfig {
    pub subject: String,
    pub issuer: String,
    pub audience: Vec<String>,
    pub scope: Option<String>,
    pub expires_in: u64,
    pub not_before: u64,
    pub claim_strings: Vec<String>,
    pub save_as: Option<String>,
    pub private_key_path: Option<PathBuf>,
}

/// Mint a test JWT token
pub fn mint_test_token(config: MintTokenConfig) -> Result<()> {
    println!("⚠️  WARNING: FOR LOCAL TESTING ONLY - DO NOT USE IN PRODUCTION\n");

    // Get current timestamp
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("Failed to get current timestamp")?
        .as_secs();

    // Calculate timestamps
    let iat = now;
    let exp = now + config.expires_in;
    let nbf = if config.not_before > 0 {
        Some(now + config.not_before)
    } else {
        None
    };

    // Parse custom claims
    let custom_claims = parse_custom_claims(config.claim_strings)?;

    // Build claims
    let claims = Claims {
        sub: config.subject.clone(),
        iss: config.issuer.clone(),
        aud: config.audience.clone(),
        exp,
        iat,
        nbf,
        scope: config.scope.clone(),
        custom: custom_claims.clone(),
    };

    // Load private key
    let key_path = if let Some(path) = config.private_key_path {
        path
    } else {
        get_jwt_test_dir()?.join("private.pem")
    };

    if !key_path.exists() {
        anyhow::bail!(
            "Private key not found at {}.\nRun 'wasmcp jwt generate-keypair' first.",
            key_path.display()
        );
    }

    let private_key_pem = fs::read_to_string(&key_path)
        .with_context(|| format!("Failed to read private key from {}", key_path.display()))?;

    let encoding_key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
        .context("Failed to parse private key")?;

    // Create header
    let header = Header::new(JWT_ALGORITHM);

    // Encode JWT
    let token = encode(&header, &claims, &encoding_key).context("Failed to encode JWT")?;

    // Display token and claims
    println!("Test JWT Token (valid for {}):", time_until_expiration(exp));
    println!("  {}\n", token);

    println!("Standard Claims:");
    println!("  sub: {}", config.subject);
    println!("  iss: {}", config.issuer);
    println!("  iat: {} ({})", iat, format_timestamp(iat));
    println!(
        "  exp: {} ({}, expires in {})",
        exp,
        format_timestamp(exp),
        time_until_expiration(exp)
    );

    if !config.audience.is_empty() {
        println!("  aud: {}", config.audience.join(", "));
    }

    if let Some(nbf_val) = nbf {
        println!("  nbf: {} ({})", nbf_val, format_timestamp(nbf_val));
    }

    if let Some(scope_val) = &config.scope {
        println!("  scope: {}", scope_val);
    }

    if !custom_claims.is_empty() {
        println!("\nCustom Claims:");
        for (key, value) in &custom_claims {
            let value_str = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => serde_json::to_string(value)
                    .unwrap_or_else(|e| format!("(serialization failed: {})", e)),
            };
            println!("  {}: {}", key, value_str);
        }
    }

    // Save if requested
    if let Some(ref name) = config.save_as {
        crate::commands::jwt::storage::save_token(name, &token, &claims)?;
        println!("\nSaved to: ~/.wasmcp/jwt-test/tokens/{}.json", name);
    }

    println!("\nExport for use:");
    println!("  export JWT_TOKEN=\"{}\"", token);

    if let Some(ref name) = config.save_as {
        println!("\nOr load from saved token:");
        println!("  export JWT_TOKEN=$(wasmcp jwt load-token {})", name);
    }

    Ok(())
}
