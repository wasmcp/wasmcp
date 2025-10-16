//! Configuration schema for wasmcp
//!
//! This module defines the data structures for wasmcp configuration.
//! The design prioritizes extensibility and backward compatibility.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Main configuration structure for wasmcp
///
/// Design for extensibility:
/// - All fields use #[serde(default)] to allow partial configs
/// - All fields use #[serde(skip_serializing_if)] to avoid cluttering output
/// - New top-level sections can be added without breaking existing configs
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WasmcpConfig {
    /// Component aliases: short name → spec (path or registry)
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub components: HashMap<String, String>,

    /// Compose profiles: reusable pipeline definitions
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub profiles: HashMap<String, Profile>,
}

/// Profile definition for compose operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Optional base profile to inherit from
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub base: Option<String>,

    /// Components in this profile (can be aliases, paths, or registry specs)
    pub components: Vec<String>,

    /// Output filename (saved to ~/.config/wasmcp/composed/ by default)
    pub output: String,
}

fn is_false(b: &bool) -> bool {
    !b
}

impl WasmcpConfig {
    /// Validate the configuration for common errors
    ///
    /// Returns Ok(()) if valid, or Err with a list of error messages
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate component aliases don't have cycles
        for (alias, _target) in &self.components {
            if let Err(e) = self.validate_alias_chain(alias) {
                errors.push(e);
            }
        }

        // Validate profiles
        for (name, profile) in &self.profiles {
            if let Err(e) = self.validate_profile(name, profile) {
                errors.push(e);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate that an alias chain doesn't contain cycles
    fn validate_alias_chain(&self, alias: &str) -> Result<(), String> {
        let mut visited = HashSet::new();
        let mut current = alias;

        while let Some(target) = self.components.get(current) {
            if !visited.insert(current) {
                return Err(format!("Circular alias dependency detected: {}", alias));
            }
            // If target looks like another alias (no path separators or registry spec), continue
            if !target.contains('/') && !target.contains('\\') && !target.ends_with(".wasm") && !target.contains(':') {
                current = target;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Validate that a profile doesn't have circular base dependencies
    fn validate_profile(&self, name: &str, profile: &Profile) -> Result<(), String> {
        // Check for circular base dependencies
        if let Some(base) = &profile.base {
            let mut visited = HashSet::new();
            let mut current = base.as_str();
            visited.insert(name);

            while let Some(p) = self.profiles.get(current) {
                if !visited.insert(current) {
                    return Err(format!("Circular profile dependency detected: {}", name));
                }
                if let Some(next_base) = &p.base {
                    current = next_base;
                } else {
                    break;
                }
            }

            // Check that base profile exists
            if !self.profiles.contains_key(base.as_str()) {
                return Err(format!(
                    "Profile '{}' inherits from non-existent base '{}'",
                    name, base
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_alias_chain() {
        let mut config = WasmcpConfig::default();

        // Valid chain
        config
            .components
            .insert("a".to_string(), "wasmcp:foo@1.0".to_string());
        config.components.insert("b".to_string(), "a".to_string());
        assert!(config.validate().is_ok());

        // Circular chain
        config.components.insert("a".to_string(), "b".to_string());
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .iter()
            .any(|e| e.contains("Circular alias dependency")));
    }

    #[test]
    fn test_validate_profile_base() {
        let mut config = WasmcpConfig::default();

        // Valid base
        config.profiles.insert(
            "base".to_string(),
            Profile {
                base: None,
                components: vec!["a".to_string()],
                output: "out.wasm".to_string(),
            },
        );
        config.profiles.insert(
            "derived".to_string(),
            Profile {
                base: Some("base".to_string()),
                components: vec!["b".to_string()],
                output: "out.wasm".to_string(),
            },
        );
        assert!(config.validate().is_ok());

        // Non-existent base
        config.profiles.insert(
            "bad".to_string(),
            Profile {
                base: Some("nonexistent".to_string()),
                components: vec![],
                output: "out.wasm".to_string(),
            },
        );
        let result = config.validate();
        assert!(result.is_err());
    }
}
