//! Input validation for compose command
//!
//! Validates transport types, output paths, and file permissions.
//! This module contains pure validation functions with no external dependencies,
//! making them straightforward to test and reuse across composition modes.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Valid transport types supported by the compose command
///
/// - `http`: Creates a server component that runs via `wasmtime serve`
/// - `stdio`: Creates a CLI component that runs via `wasmtime run`
pub const VALID_TRANSPORTS: &[&str] = &["http", "stdio"];

/// Validate transport type is supported
///
/// # Arguments
///
/// * `transport` - The transport type to validate ("http" or "stdio")
///
/// # Errors
///
/// Returns an error if the transport type is not in [`VALID_TRANSPORTS`]
///
/// # Examples
///
/// ```rust,ignore
/// # use wasmcp::commands::compose::validation::validate_transport;
/// assert!(validate_transport("http").is_ok());
/// assert!(validate_transport("stdio").is_ok());
/// assert!(validate_transport("websocket").is_err());
/// ```
pub fn validate_transport(transport: &str) -> Result<()> {
    if !VALID_TRANSPORTS.contains(&transport) {
        anyhow::bail!(
            "unsupported transport type: '{}', must be one of: {}",
            transport,
            VALID_TRANSPORTS.join(", ")
        );
    }
    Ok(())
}

/// Resolve output path - make absolute if relative (using current working directory)
///
/// # Arguments
///
/// * `output` - The output path which may be relative or absolute
///
/// # Errors
///
/// Returns an error if unable to determine the current working directory
///
/// # Examples
///
/// ```rust,ignore
/// # use std::path::PathBuf;
/// # use wasmcp::commands::compose::validation::resolve_output_path;
/// # fn example() -> anyhow::Result<()> {
/// let relative = PathBuf::from("output.wasm");
/// let absolute = resolve_output_path(&relative)?;
/// assert!(absolute.is_absolute());
///
/// let already_absolute = PathBuf::from("/tmp/output.wasm");
/// let resolved = resolve_output_path(&already_absolute)?;
/// assert_eq!(resolved, already_absolute);
/// # Ok(())
/// # }
/// ```
pub fn resolve_output_path(output: &PathBuf) -> Result<PathBuf> {
    if output.is_absolute() {
        Ok(output.clone())
    } else {
        // Resolve relative paths against current working directory
        let cwd = std::env::current_dir().context("Failed to get current working directory")?;
        Ok(cwd.join(output))
    }
}

/// Validate output file doesn't exist (unless force is set)
///
/// Also validates that the parent directory exists and is writable.
///
/// # Arguments
///
/// * `output_path` - The resolved absolute path where output will be written
/// * `force` - If true, allow overwriting existing files
///
/// # Errors
///
/// Returns an error if:
/// - The output file exists and `force` is false
/// - The parent directory doesn't exist
/// - The parent directory is not writable
/// - Unable to read directory metadata
///
/// # Examples
///
/// ```rust,ignore
/// # use std::path::Path;
/// # use wasmcp::commands::compose::validation::validate_output_file;
/// # fn example() -> anyhow::Result<()> {
/// let path = Path::new("/tmp/new-file.wasm");
///
/// // Allow writing new files
/// validate_output_file(path, false)?;
///
/// // Force overwrite existing files
/// validate_output_file(path, true)?;
/// # Ok(())
/// # }
/// ```
pub fn validate_output_file(output_path: &Path, force: bool) -> Result<()> {
    if output_path.exists() && !force {
        anyhow::bail!(
            "output file '{}' already exists, use --force to overwrite",
            output_path.display()
        );
    }

    // Check parent directory is writable
    if let Some(parent) = output_path.parent() {
        if parent.exists() {
            let metadata = std::fs::metadata(parent).context(format!(
                "failed to read directory metadata for '{}'",
                parent.display()
            ))?;
            if metadata.permissions().readonly() {
                anyhow::bail!("output directory '{}' is not writable", parent.display());
            }
        } else {
            anyhow::bail!("output directory '{}' does not exist", parent.display());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_transport_http() {
        let result = validate_transport("http");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_transport_stdio() {
        let result = validate_transport("stdio");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_transport_invalid() {
        let result = validate_transport("websocket");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("unsupported transport type"));
        assert!(err_msg.contains("websocket"));
        assert!(err_msg.contains("http"));
        assert!(err_msg.contains("stdio"));
    }

    /// Test VALID_TRANSPORTS constant
    #[test]
    fn test_valid_transports_constant() {
        assert_eq!(VALID_TRANSPORTS, &["http", "stdio"]);
        assert_eq!(VALID_TRANSPORTS.len(), 2);
        assert!(VALID_TRANSPORTS.contains(&"http"));
        assert!(VALID_TRANSPORTS.contains(&"stdio"));
    }
}
