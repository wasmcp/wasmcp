use anyhow::{Context, Result};
use std::path::PathBuf;

/// Find the workspace root by looking for Cargo.toml with [workspace]
pub fn find_workspace_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir().context("Failed to get current directory")?;

    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            let content =
                std::fs::read_to_string(&cargo_toml).context("Failed to read Cargo.toml")?;

            // Check if this is a workspace root
            if content.contains("[workspace]") {
                return Ok(current);
            }
        }

        // Try parent directory
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            anyhow::bail!("Could not find workspace root (no Cargo.toml with [workspace])");
        }
    }
}

/// Find the git repository root by looking for .git directory
pub fn find_repo_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?.canonicalize()?;

    loop {
        if current.join(".git").exists() {
            return Ok(current);
        }

        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            anyhow::bail!("Could not find repository root (no .git directory found)");
        }
    }
}
