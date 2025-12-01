//! JWT testing utilities for local development
//!
//! Provides commands for generating RSA keypairs and minting test JWT tokens
//! with custom claims for testing authentication and authorization patterns.

mod decode;
mod keygen;
mod mint;
mod storage;

use anyhow::Result;
use chrono::DateTime;
use clap::Parser;
use std::path::PathBuf;

/// Format timestamp as human-readable UTC string
pub(crate) fn format_timestamp(timestamp: u64) -> String {
    if let Some(datetime) = DateTime::from_timestamp(timestamp as i64, 0) {
        datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    } else {
        "Invalid timestamp".to_string()
    }
}

/// Get the JWT test directory (~/.wasmcp/jwt-test/)
pub(crate) fn get_jwt_test_dir() -> Result<PathBuf> {
    use anyhow::Context;
    let wasmcp_dir = crate::config::get_wasmcp_dir().context("Failed to get wasmcp directory")?;
    Ok(wasmcp_dir.join("jwt-test"))
}

#[derive(Parser)]
pub enum JwtCommand {
    /// Generate RSA keypair for JWT testing
    ///
    /// Creates a new RSA-2048 keypair in ~/.wasmcp/jwt-test/ for local JWT testing.
    /// The public key can be used with JWT_PUBLIC_KEY environment variable.
    ///
    /// ⚠️  WARNING: FOR LOCAL TESTING ONLY - DO NOT USE IN PRODUCTION
    GenerateKeypair {
        /// Overwrite existing keypair if it exists
        #[arg(long)]
        force: bool,
    },

    /// Mint a JWT token with custom claims
    ///
    /// Creates a JWT token signed with the test private key. Supports custom
    /// claims for testing authorization patterns (scopes, roles, permissions, etc.).
    ///
    /// ⚠️  WARNING: FOR LOCAL TESTING ONLY - DO NOT USE IN PRODUCTION
    Mint {
        /// Subject claim (user identifier)
        #[arg(long, default_value = "test-user")]
        subject: String,

        /// Issuer claim
        #[arg(long, default_value = "wasmcp-local-test")]
        issuer: String,

        /// Audience claim (can be specified multiple times)
        #[arg(long)]
        audience: Vec<String>,

        /// Scope claim (space-separated OAuth scopes)
        #[arg(long)]
        scope: Option<String>,

        /// Token expiration in seconds from now
        #[arg(long, default_value = "3600")]
        expires_in: u64,

        /// Not-before claim in seconds from now
        #[arg(long, default_value = "0")]
        not_before: u64,

        /// Custom claim in key=value format (can be specified multiple times)
        #[arg(long)]
        claim: Vec<String>,

        /// Save token with this name to ~/.wasmcp/jwt-test/tokens/
        #[arg(long)]
        save_as: Option<String>,

        /// Path to private key (defaults to ~/.wasmcp/jwt-test/private.pem)
        #[arg(long)]
        private_key: Option<PathBuf>,
    },

    /// Load a saved token
    ///
    /// Outputs the raw JWT token for use with export or curl.
    ///
    /// Example:
    ///   export JWT_TOKEN=$(wasmcp jwt load-token admin)
    LoadToken {
        /// Name of saved token
        name: String,
    },

    /// List all saved tokens
    ///
    /// Shows saved tokens with their subject, expiration, and status.
    ListTokens,

    /// Decode and display token contents
    ///
    /// Accepts either a raw JWT token string or a saved token name.
    /// Displays header, claims, and validation status.
    DecodeToken {
        /// Token string or saved token name
        token: String,
    },
}

pub async fn handle_jwt_command(command: JwtCommand) -> Result<()> {
    match command {
        JwtCommand::GenerateKeypair { force } => keygen::generate_keypair(force),

        JwtCommand::Mint {
            subject,
            issuer,
            audience,
            scope,
            expires_in,
            not_before,
            claim,
            save_as,
            private_key,
        } => mint::mint_test_token(mint::MintTokenConfig {
            subject,
            issuer,
            audience,
            scope,
            expires_in,
            not_before,
            claim_strings: claim,
            save_as,
            private_key_path: private_key,
        }),

        JwtCommand::LoadToken { name } => storage::load_token(&name),

        JwtCommand::ListTokens => storage::list_tokens(),

        JwtCommand::DecodeToken { token } => decode::decode_token(&token),
    }
}
