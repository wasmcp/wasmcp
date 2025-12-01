//! Token storage and retrieval

use anyhow::{Context, Result};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::get_jwt_test_dir;

/// Stored token metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct StoredToken {
    pub name: String,
    pub token: String,
    pub claims: StoredClaims,
    pub created_at: String,
}

/// Claims stored with token
#[derive(Debug, Serialize, Deserialize)]
pub struct StoredClaims {
    pub sub: String,
    pub iss: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub aud: Vec<String>,
    pub exp: u64,
    pub iat: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nbf: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(flatten)]
    pub custom: HashMap<String, Value>,
}

/// Get the tokens directory
fn get_tokens_dir() -> Result<PathBuf> {
    Ok(get_jwt_test_dir()?.join("tokens"))
}

/// Save a token with metadata
pub fn save_token(name: &str, token: &str, claims: &super::mint::Claims) -> Result<()> {
    let tokens_dir = get_tokens_dir()?;
    fs::create_dir_all(&tokens_dir)
        .with_context(|| format!("Failed to create directory: {}", tokens_dir.display()))?;

    let token_path = tokens_dir.join(format!("{}.json", name));

    // Convert claims to stored format
    let stored_claims = StoredClaims {
        sub: claims.sub.clone(),
        iss: claims.iss.clone(),
        aud: claims.aud.clone(),
        exp: claims.exp,
        iat: claims.iat,
        nbf: claims.nbf,
        scope: claims.scope.clone(),
        custom: claims.custom.clone(),
    };

    let stored_token = StoredToken {
        name: name.to_string(),
        token: token.to_string(),
        claims: stored_claims,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let json = serde_json::to_string_pretty(&stored_token).context("Failed to serialize token")?;

    fs::write(&token_path, json)
        .with_context(|| format!("Failed to write token to {}", token_path.display()))?;

    Ok(())
}

/// Load a token by name (outputs raw token for export)
pub fn load_token(name: &str) -> Result<()> {
    let tokens_dir = get_tokens_dir()?;
    let token_path = tokens_dir.join(format!("{}.json", name));

    if !token_path.exists() {
        anyhow::bail!(
            "Token '{}' not found.\nRun 'wasmcp jwt list-tokens' to see available tokens.",
            name
        );
    }

    let json = fs::read_to_string(&token_path)
        .with_context(|| format!("Failed to read token from {}", token_path.display()))?;

    let stored: StoredToken =
        serde_json::from_str(&json).context("Failed to parse stored token")?;

    // Output raw token for use with export
    println!("{}", stored.token);

    Ok(())
}

/// Try to load a token from a file path
fn try_load_token(path: &std::path::Path) -> Option<StoredToken> {
    if path.extension()?.to_str()? != "json" {
        return None;
    }
    let json = fs::read_to_string(path).ok()?;
    serde_json::from_str(&json).ok()
}

/// List all saved tokens
pub fn list_tokens() -> Result<()> {
    let tokens_dir = get_tokens_dir()?;

    if !tokens_dir.exists() {
        println!("No saved tokens found.");
        println!("\nTo create a token, use:");
        println!("  wasmcp jwt mint --save-as <name>");
        return Ok(());
    }

    let entries = fs::read_dir(&tokens_dir)
        .with_context(|| format!("Failed to read directory: {}", tokens_dir.display()))?;

    let mut tokens = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if let Some(stored) = try_load_token(&path) {
            tokens.push(stored);
        }
    }

    if tokens.is_empty() {
        println!("No saved tokens found.");
        println!("\nTo create a token, use:");
        println!("  wasmcp jwt mint --save-as <name>");
        return Ok(());
    }

    println!("Saved test tokens (~/.wasmcp/jwt-test/tokens/):\n");

    // Get current time for expiration checks
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("Failed to get current timestamp")?
        .as_secs();

    // Sort by name
    tokens.sort_by(|a, b| a.name.cmp(&b.name));

    for token in tokens {
        let exp_time = DateTime::from_timestamp(token.claims.exp as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "Invalid timestamp".to_string());

        let status = if token.claims.exp <= now {
            "EXPIRED"
        } else {
            "valid"
        };

        println!(
            "  {}  ({}, expires: {}) [{}]",
            token.name, token.claims.sub, exp_time, status
        );
    }

    Ok(())
}

/// Load stored token for decoding
pub fn load_stored_token(name: &str) -> Result<String> {
    let tokens_dir = get_tokens_dir()?;
    let token_path = tokens_dir.join(format!("{}.json", name));

    if !token_path.exists() {
        anyhow::bail!("Token '{}' not found", name);
    }

    let json = fs::read_to_string(&token_path)?;
    let stored: StoredToken = serde_json::from_str(&json)?;

    Ok(stored.token)
}
