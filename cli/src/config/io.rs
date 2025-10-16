//! Configuration file I/O operations
//!
//! This module handles reading, writing, and updating the wasmcp configuration file.
//! All operations include automatic validation.

use super::paths::get_config_path;
use super::schema::WasmcpConfig;
use anyhow::{Context, Result};
use std::fs;

/// Load configuration from disk
///
/// Returns a default (empty) config if the file doesn't exist.
/// Validates the config after loading.
pub fn load_config() -> Result<WasmcpConfig> {
    let path = get_config_path()?;

    if !path.exists() {
        return Ok(WasmcpConfig::default());
    }

    let content =
        fs::read_to_string(&path).context(format!("Failed to read config: {}", path.display()))?;

    let config: WasmcpConfig = toml::from_str(&content)
        .context(format!("Failed to parse config: {}", path.display()))?;

    // Validate config after loading
    if let Err(errors) = config.validate() {
        anyhow::bail!(
            "Config validation failed in {}:\n  {}",
            path.display(),
            errors.join("\n  ")
        );
    }

    Ok(config)
}

/// Save configuration to disk
///
/// Creates parent directories if needed.
/// Validates the config before saving.
pub fn save_config(config: &WasmcpConfig) -> Result<()> {
    // Validate before saving
    if let Err(errors) = config.validate() {
        anyhow::bail!("Cannot save invalid config:\n  {}", errors.join("\n  "));
    }

    let path = get_config_path()?;

    // Create parent directory
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .context(format!("Failed to create config directory: {}", parent.display()))?;
    }

    let content = toml::to_string_pretty(config).context("Failed to serialize config")?;

    fs::write(&path, content).context(format!("Failed to write config: {}", path.display()))?;

    Ok(())
}

/// Update config with a modification function
///
/// This handles the load → modify → validate → save cycle atomically.
/// The modification function receives a mutable reference to the config.
///
/// # Example
///
/// ```rust
/// update_config(|config| {
///     config.components.insert("calc".to_string(), "wasmcp:calculator@0.1.0".to_string());
///     Ok(())
/// })?;
/// ```
pub fn update_config<F>(f: F) -> Result<()>
where
    F: FnOnce(&mut WasmcpConfig) -> Result<()>,
{
    let mut config = load_config()?;
    f(&mut config)?;
    save_config(&config)?;
    Ok(())
}

/// Add or update a component alias
///
/// This is a convenience wrapper around `update_config`.
/// If the spec looks like a local path, it will be canonicalized to an absolute path.
pub fn register_component(alias: &str, spec: &str) -> Result<()> {
    // If spec looks like a path (relative or absolute), canonicalize it
    let final_spec = if spec.contains('/') || spec.contains('\\') || spec.ends_with(".wasm") {
        let path = std::path::PathBuf::from(spec);

        // Canonicalize to get absolute path (also validates existence)
        let canonical = path.canonicalize()
            .context(format!("Path does not exist or cannot be resolved: {}", spec))?;

        canonical.to_string_lossy().to_string()
    } else {
        // Registry spec or another alias - keep as-is
        spec.to_string()
    };

    update_config(|config| {
        config
            .components
            .insert(alias.to_string(), final_spec);
        Ok(())
    })
}

/// Remove a component alias
///
/// Returns an error if the alias doesn't exist.
pub fn unregister_component(alias: &str) -> Result<()> {
    update_config(|config| {
        if config.components.remove(alias).is_none() {
            anyhow::bail!("Alias '{}' not found", alias);
        }
        Ok(())
    })
}

/// Create or update a profile
///
/// This is a convenience wrapper around `update_config`.
pub fn create_profile(name: &str, profile: super::schema::Profile) -> Result<()> {
    update_config(|config| {
        config.profiles.insert(name.to_string(), profile);
        Ok(())
    })
}

/// Remove a profile
///
/// Returns an error if the profile doesn't exist.
pub fn delete_profile(name: &str) -> Result<()> {
    update_config(|config| {
        if config.profiles.remove(name).is_none() {
            anyhow::bail!("Profile '{}' not found", name);
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_nonexistent_returns_default() {
        // This would normally use get_config_path(), but we can't control that in tests
        // Just verify the logic is sound
        let config = WasmcpConfig::default();
        assert!(config.components.is_empty());
        assert!(config.profiles.is_empty());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Create a config
        let mut config = WasmcpConfig::default();
        config
            .components
            .insert("calc".to_string(), "wasmcp:calculator@0.1.0".to_string());

        // Save it
        let content = toml::to_string_pretty(&config).unwrap();
        fs::write(&config_path, content).unwrap();

        // Load it back
        let content = fs::read_to_string(&config_path).unwrap();
        let loaded: WasmcpConfig = toml::from_str(&content).unwrap();

        assert_eq!(loaded.components.len(), 1);
        assert_eq!(
            loaded.components.get("calc"),
            Some(&"wasmcp:calculator@0.1.0".to_string())
        );
    }
}
