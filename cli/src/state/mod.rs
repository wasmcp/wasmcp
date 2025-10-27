//! XDG state directory management for wasmcp
//!
//! Handles platform-specific state directory locations for PID files,
//! log files, and other runtime state.
//!
//! Uses the `dirs` crate for consistent XDG Base Directory specification support.
//!
//! ## Directory Locations
//!
//! **macOS:**
//! - `~/Library/Application Support/wasmcp/` (dirs::data_local_dir)
//!
//! **Linux/Unix:**
//! - `$XDG_STATE_HOME/wasmcp/` (if XDG_STATE_HOME is set)
//! - `~/.local/state/wasmcp/` (default fallback)
//!
//! **Windows:** (not yet supported)
//! - Would use `%LOCALAPPDATA%\wasmcp\data\` via dirs::data_local_dir

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Get the XDG state directory for wasmcp
///
/// Uses platform-appropriate locations via the `dirs` crate:
/// - macOS: ~/Library/Application Support/wasmcp/
/// - Linux: $XDG_STATE_HOME/wasmcp/ or ~/.local/state/wasmcp/
/// - Windows: %LOCALAPPDATA%\wasmcp\data\ (not yet supported)
pub fn get_state_dir() -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        // macOS uses data_local_dir which maps to ~/Library/Application Support
        dirs::data_local_dir()
            .context("Failed to get data local directory")?
            .join("wasmcp")
            .pipe(Ok)
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // Linux/Unix: Use XDG_STATE_HOME if set, otherwise ~/.local/state
        // The dirs crate doesn't have state_dir(), so we implement XDG spec manually
        if let Ok(xdg_state) = std::env::var("XDG_STATE_HOME") {
            Ok(PathBuf::from(xdg_state).join("wasmcp"))
        } else {
            dirs::home_dir()
                .context("Failed to get home directory")?
                .join(".local")
                .join("state")
                .join("wasmcp")
                .pipe(Ok)
        }
    }

    #[cfg(not(unix))]
    {
        // Windows: Not yet supported
        anyhow::bail!(
            "Windows daemon mode not yet implemented. Use 'wasmcp mcp serve' for foreground mode."
        )
    }
}

// Utility trait for Result<PathBuf> → Ok(PathBuf)
trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

impl Pipe for PathBuf {}

/// Ensure the state directory exists, creating it if necessary
pub fn ensure_state_dir() -> Result<PathBuf> {
    let state_dir = get_state_dir()?;
    std::fs::create_dir_all(&state_dir)
        .with_context(|| format!("Failed to create state directory: {}", state_dir.display()))?;
    Ok(state_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_state_dir() {
        let state_dir = get_state_dir().expect("Failed to get state dir");

        #[cfg(target_os = "macos")]
        assert!(state_dir.ends_with("Library/Application Support/wasmcp"));

        #[cfg(not(target_os = "macos"))]
        {
            let dir_str = state_dir.to_str().expect("Invalid UTF-8 in path");
            assert!(dir_str.contains(".local/state/wasmcp") || dir_str.ends_with("/wasmcp"));
        }
    }

    #[test]
    fn test_ensure_state_dir() {
        let state_dir = ensure_state_dir().expect("Failed to ensure state dir");
        assert!(state_dir.exists());
        assert!(state_dir.is_dir());
    }
}
