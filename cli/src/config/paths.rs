//! Directory path management for wasmcp
//!
//! All paths used by wasmcp are centralized here. This makes it easy to
//! understand and modify the directory structure.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Get the base wasmcp directory (~/.config/wasmcp/)
///
/// This is the root directory for all wasmcp data.
pub fn get_wasmcp_dir() -> Result<PathBuf> {
    dirs::config_dir()
        .context("Failed to get config directory")?
        .join("wasmcp")
        .pipe(Ok)
}

/// Get the config file path (~/.config/wasmcp/config.toml)
pub fn get_config_path() -> Result<PathBuf> {
    Ok(get_wasmcp_dir()?.join("config.toml"))
}

/// Get the deps directory (~/.config/wasmcp/deps/)
///
/// This is where downloaded registry components are stored.
pub fn get_deps_dir() -> Result<PathBuf> {
    Ok(get_wasmcp_dir()?.join("deps"))
}

/// Get the cache directory (~/.config/wasmcp/cache/)
///
/// This is where wasm-pkg-client stores its cache.
pub fn get_cache_dir() -> Result<PathBuf> {
    Ok(get_wasmcp_dir()?.join("cache"))
}

/// Get the composed directory (~/.config/wasmcp/composed/)
///
/// This is the default output directory for composed servers.
pub fn get_composed_dir() -> Result<PathBuf> {
    Ok(get_wasmcp_dir()?.join("composed"))
}

/// Ensure all wasmcp directories exist
///
/// Creates the directory structure if it doesn't exist.
/// Safe to call multiple times.
pub fn ensure_dirs() -> Result<()> {
    for dir in [get_deps_dir()?, get_cache_dir()?, get_composed_dir()?] {
        std::fs::create_dir_all(&dir)
            .context(format!("Failed to create directory: {}", dir.display()))?;
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasmcp_dir_is_under_config() {
        let dir = get_wasmcp_dir().unwrap();
        assert!(dir.ends_with("wasmcp"));
        // Should be under the system config directory (platform-specific)
        let parent = dir.parent().unwrap();
        let config_dir = dirs::config_dir().unwrap();
        assert_eq!(
            parent, config_dir,
            "wasmcp dir should be directly under system config directory"
        );
    }

    #[test]
    fn test_subdirs_are_correct() {
        let base = get_wasmcp_dir().unwrap();
        assert_eq!(get_deps_dir().unwrap(), base.join("deps"));
        assert_eq!(get_cache_dir().unwrap(), base.join("cache"));
        assert_eq!(get_composed_dir().unwrap(), base.join("composed"));
        assert_eq!(get_config_path().unwrap(), base.join("config.toml"));
    }
}
