//! RSA keypair generation for JWT testing

use anyhow::{Context, Result};
use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
use rsa::{RsaPrivateKey, RsaPublicKey};
use std::fs;

use super::get_jwt_test_dir;

/// RSA key size for test keypairs (2048-bit minimum for JWT RS256)
const RSA_KEY_BITS: usize = 2048;

/// Generate RSA keypair for JWT testing
pub fn generate_keypair(force: bool) -> Result<()> {
    println!("⚠️  WARNING: FOR LOCAL TESTING ONLY - DO NOT USE IN PRODUCTION\n");

    let jwt_dir = get_jwt_test_dir()?;
    let private_key_path = jwt_dir.join("private.pem");
    let public_key_path = jwt_dir.join("public.pem");

    // Check if keys already exist
    if (private_key_path.exists() || public_key_path.exists()) && !force {
        anyhow::bail!(
            "Keypair already exists in {}.\nUse --force to overwrite.",
            jwt_dir.display()
        );
    }

    // Create directory if it doesn't exist
    fs::create_dir_all(&jwt_dir)
        .with_context(|| format!("Failed to create directory: {}", jwt_dir.display()))?;

    // Generate RSA-2048 keypair
    println!("Generating RSA-2048 keypair...");
    let mut rng = rand::thread_rng();
    let bits = RSA_KEY_BITS;
    let private_key =
        RsaPrivateKey::new(&mut rng, bits).context("Failed to generate private key")?;
    let public_key = RsaPublicKey::from(&private_key);

    // Encode to PEM format
    let private_pem = private_key
        .to_pkcs8_pem(LineEnding::LF)
        .context("Failed to encode private key to PEM")?;
    let public_pem = public_key
        .to_public_key_pem(LineEnding::LF)
        .context("Failed to encode public key to PEM")?;

    // Write keys to files
    fs::write(&private_key_path, private_pem.as_bytes()).with_context(|| {
        format!(
            "Failed to write private key to {}",
            private_key_path.display()
        )
    })?;
    fs::write(&public_key_path, public_pem.as_bytes()).with_context(|| {
        format!(
            "Failed to write public key to {}",
            public_key_path.display()
        )
    })?;

    // Set restrictive permissions on private key (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&private_key_path)?.permissions();
        perms.set_mode(0o600); // rw-------
        fs::set_permissions(&private_key_path, perms)?;
    }

    #[cfg(not(unix))]
    {
        println!("⚠️  WARNING: Cannot set restrictive permissions on this platform.");
        println!("   Ensure private key is stored securely.");
        println!();
    }

    println!("Generated RSA-2048 keypair in {}:", jwt_dir.display());
    println!("  Private key: {}", private_key_path.display());
    println!("  Public key:  {}", public_key_path.display());
    println!();
    println!("To use with wasmcp servers, set:");
    println!(
        "  export JWT_PUBLIC_KEY=\"$(cat {})\"",
        public_key_path.display()
    );
    println!("  export JWT_ISSUER=\"wasmcp-local-test\"");
    println!("  export WASMCP_AUTH_MODE=\"oauth\"");

    Ok(())
}
