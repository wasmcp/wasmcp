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

    /// Apply version overrides from CLI arguments (legacy Vec<String> format)
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

    /// Apply version overrides from pre-parsed HashMap
    pub fn apply_override_map(&mut self, overrides: &HashMap<String, String>) -> Result<()> {
        self.overrides.extend(overrides.clone());
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

    /// Get all valid component names (from versions.toml)
    ///
    /// Returns component names only, excluding WIT packages like mcp-v20250618
    pub fn get_component_names(&self) -> Vec<String> {
        self.versions
            .keys()
            .filter(|name| !name.contains("v202") && *name != "mcp-v20250618")
            .cloned()
            .collect()
    }

    /// Check if a component name is valid
    pub fn is_valid_component(&self, name: &str) -> bool {
        self.versions.contains_key(name) && !name.contains("v202") && name != "mcp-v20250618"
    }

    /// Get comma-separated list of valid component names
    pub fn valid_components_list(&self) -> String {
        let mut names = self.get_component_names();
        names.sort();
        names.join(", ")
    }

    /// Get all framework component names (excludes spec versions)
    ///
    /// Returns all components from versions.toml that are actual framework components,
    /// not WIT package specs like mcp-v20250618.
    pub fn framework_components(&self) -> Vec<&str> {
        self.versions
            .keys()
            .filter(|name| !name.contains("v202") && name.as_str() != "mcp-v20250618")
            .map(|s| s.as_str())
            .collect()
    }

    /// Get service component names (dynamic, non-structural components)
    ///
    /// Returns components that should be registered in the ServiceRegistry:
    /// - Excludes structural components (transport, method-not-found)
    /// - Excludes middleware components (*-middleware)
    ///
    /// These components are instantiated and auto-wired based on imports.
    pub fn service_components(&self) -> Vec<&str> {
        self.framework_components()
            .into_iter()
            .filter(|name| !self.is_structural(name) && !name.ends_with("-middleware"))
            .collect()
    }

    /// Get middleware component names (capability wrappers)
    ///
    /// Returns components whose names end with "-middleware".
    /// These are used for wrapping capability components.
    pub fn middleware_components(&self) -> Vec<&str> {
        self.framework_components()
            .into_iter()
            .filter(|name| name.ends_with("-middleware"))
            .collect()
    }

    /// Check if a component is structural (fixed pipeline position)
    ///
    /// Structural components:
    /// - transport: Always at front of pipeline, exports WASI interface
    /// - method-not-found: Always at end of pipeline, terminal handler
    pub fn is_structural(&self, name: &str) -> bool {
        name == "transport" || name == "method-not-found"
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

    #[test]
    fn test_apply_override_map() {
        let mut resolver = VersionResolver::new().unwrap();

        let mut overrides = HashMap::new();
        overrides.insert("transport".to_string(), "0.2.0".to_string());
        overrides.insert("server-io".to_string(), "0.3.0".to_string());

        resolver.apply_override_map(&overrides).unwrap();

        assert_eq!(resolver.get_version("transport").unwrap(), "0.2.0");
        assert_eq!(resolver.get_version("server-io").unwrap(), "0.3.0");
    }

    #[test]
    fn test_apply_override_map_empty() {
        let mut resolver = VersionResolver::new().unwrap();

        let overrides = HashMap::new();
        resolver.apply_override_map(&overrides).unwrap();

        // Should still get default versions
        assert!(resolver.get_version("transport").is_ok());
    }

    #[test]
    fn test_apply_override_map_multiple_calls() {
        let mut resolver = VersionResolver::new().unwrap();

        let mut overrides1 = HashMap::new();
        overrides1.insert("transport".to_string(), "0.2.0".to_string());

        let mut overrides2 = HashMap::new();
        overrides2.insert("server-io".to_string(), "0.3.0".to_string());
        overrides2.insert("transport".to_string(), "0.4.0".to_string());

        resolver.apply_override_map(&overrides1).unwrap();
        resolver.apply_override_map(&overrides2).unwrap();

        // Second call should override first
        assert_eq!(resolver.get_version("transport").unwrap(), "0.4.0");
        assert_eq!(resolver.get_version("server-io").unwrap(), "0.3.0");
    }

    #[test]
    fn test_framework_components() {
        let resolver = VersionResolver::new().unwrap();
        let components = resolver.framework_components();

        // Should include actual components
        assert!(components.contains(&"transport"));
        assert!(components.contains(&"server-io"));
        assert!(components.contains(&"method-not-found"));

        // Should exclude spec versions
        assert!(!components.contains(&"mcp-v20250618"));
        assert!(!components.iter().any(|c| c.contains("v202")));
    }

    #[test]
    fn test_service_components() {
        let resolver = VersionResolver::new().unwrap();
        let services = resolver.service_components();

        // Should include service components
        assert!(services.contains(&"server-io"));
        assert!(services.contains(&"authorization"));
        assert!(services.contains(&"kv-store"));
        assert!(services.contains(&"session-store"));

        // Should exclude structural components
        assert!(!services.contains(&"transport"));
        assert!(!services.contains(&"method-not-found"));

        // Should exclude middleware
        assert!(!services.contains(&"tools-middleware"));
        assert!(!services.contains(&"resources-middleware"));
        assert!(!services.contains(&"prompts-middleware"));
    }

    #[test]
    fn test_middleware_components() {
        let resolver = VersionResolver::new().unwrap();
        let middleware = resolver.middleware_components();

        // Should include middleware components
        assert!(middleware.contains(&"tools-middleware"));
        assert!(middleware.contains(&"resources-middleware"));
        assert!(middleware.contains(&"prompts-middleware"));

        // Should not include non-middleware
        assert!(!middleware.contains(&"transport"));
        assert!(!middleware.contains(&"server-io"));
    }

    #[test]
    fn test_is_structural() {
        let resolver = VersionResolver::new().unwrap();

        // Structural components
        assert!(resolver.is_structural("transport"));
        assert!(resolver.is_structural("method-not-found"));

        // Non-structural components
        assert!(!resolver.is_structural("server-io"));
        assert!(!resolver.is_structural("authorization"));
        assert!(!resolver.is_structural("tools-middleware"));
    }
}
