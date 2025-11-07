//! Version management for wasmcp components
//!
//! This module handles version resolution for WIT packages and components.
//! The WebAssembly Component Model requires exact version matching - there
//! is no concept of version compatibility or ranges.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Version manifest loaded from versions.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    /// Component and WIT package versions
    pub versions: HashMap<String, String>,
    /// WASI interface versions
    #[serde(default)]
    pub wasi: HashMap<String, String>,
}

/// Version resolver that handles version lookups and overrides
#[derive(Debug)]
pub struct VersionResolver {
    /// Base versions from manifest
    versions: HashMap<String, String>,
    /// WASI versions from manifest
    wasi: HashMap<String, String>,
    /// User-specified overrides
    overrides: HashMap<String, String>,
}

impl VersionResolver {
    /// Create a new version resolver
    pub fn new() -> Result<Self> {
        // Load embedded versions.toml
        let manifest_str = include_str!("../../versions.toml");
        let manifest: VersionManifest =
            toml::from_str(manifest_str).context("Failed to parse embedded versions.toml")?;

        Ok(Self {
            versions: manifest.versions,
            wasi: manifest.wasi,
            overrides: HashMap::new(),
        })
    }

    /// Create resolver from a versions.toml file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let manifest: VersionManifest = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;

        Ok(Self {
            versions: manifest.versions,
            wasi: manifest.wasi,
            overrides: HashMap::new(),
        })
    }

    /// Apply version overrides from CLI arguments
    pub fn apply_overrides(&mut self, overrides: Vec<String>) -> Result<()> {
        for override_str in overrides {
            let parts: Vec<&str> = override_str.split('=').collect();
            if parts.len() != 2 {
                anyhow::bail!(
                    "Invalid version override format: '{}'. Expected 'component=version'",
                    override_str
                );
            }

            let component = parts[0].to_string();
            let version = parts[1].to_string();

            self.overrides.insert(component, version);
        }
        Ok(())
    }

    /// Get the version for a component or WIT package
    pub fn get_version(&self, name: &str) -> Result<String> {
        // Check overrides first
        if let Some(version) = self.overrides.get(name) {
            return Ok(version.clone());
        }

        // Then check manifest
        self.versions
            .get(name)
            .cloned()
            .with_context(|| format!("No version found for '{}'", name))
    }

    /// Get WASI interface version
    pub fn get_wasi_version(&self, interface: &str) -> Result<String> {
        // Check overrides first (format: wasi-http, wasi-cli)
        let override_key = format!("wasi-{}", interface);
        if let Some(version) = self.overrides.get(&override_key) {
            return Ok(version.clone());
        }

        // Then check wasi section
        self.wasi
            .get(interface)
            .cloned()
            .with_context(|| format!("No WASI version found for '{}'", interface))
    }

    /// Get all resolved versions (manifest + overrides)
    pub fn get_all_versions(&self) -> HashMap<String, String> {
        let mut result = self.versions.clone();
        result.extend(self.overrides.clone());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_resolver() {
        let resolver = VersionResolver::new().unwrap();

        // Should have default versions
        assert!(resolver.get_version("mcp-v20250618").is_ok());
        assert!(resolver.get_version("transport").is_ok());
    }

    #[test]
    fn test_overrides() {
        let mut resolver = VersionResolver::new().unwrap();

        resolver
            .apply_overrides(vec!["transport=0.2.0".to_string()])
            .unwrap();

        assert_eq!(resolver.get_version("transport").unwrap(), "0.2.0");
    }

    #[test]
    fn test_invalid_override_format() {
        let mut resolver = VersionResolver::new().unwrap();

        assert!(
            resolver
                .apply_overrides(vec!["invalid".to_string()])
                .is_err()
        );
    }
}
