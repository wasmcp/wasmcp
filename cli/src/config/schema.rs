//! Configuration schema for wasmcp
//!
//! This module defines the data structures for wasmcp configuration.
//! The design prioritizes extensibility and backward compatibility.

use anyhow::Result;
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

impl WasmcpConfig {
    /// Validate the configuration for common errors
    ///
    /// Returns Ok(()) if valid, or Err with a list of error messages
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate component aliases don't have cycles
        for alias in self.components.keys() {
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
        use super::utils;

        let mut visited = HashSet::new();
        let mut chain = Vec::new();
        let mut current = alias;

        while let Some(target) = self.components.get(current) {
            chain.push(current.to_string());
            if !visited.insert(current) {
                chain.push(current.to_string()); // Add the repeated alias to show the cycle
                return Err(format!(
                    "Circular alias dependency detected: {} → {}",
                    chain.join(" → "),
                    current
                ));
            }
            // If target looks like another alias (not a path or registry spec), continue
            if !utils::is_path_spec(target) && !utils::is_registry_spec(target) {
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
            let mut chain = vec![name.to_string()];
            let mut current = base.as_str();
            visited.insert(name);

            while let Some(p) = self.profiles.get(current) {
                chain.push(current.to_string());
                if !visited.insert(current) {
                    return Err(format!(
                        "Circular profile inheritance detected: {} → {}",
                        chain.join(" → "),
                        current
                    ));
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

    #[test]
    fn test_multi_hop_alias_chain() {
        let mut config = WasmcpConfig::default();

        // Create a multi-hop chain: d → c → b → a → registry:package
        config
            .components
            .insert("a".to_string(), "wasmcp:calculator@1.0".to_string());
        config.components.insert("b".to_string(), "a".to_string());
        config.components.insert("c".to_string(), "b".to_string());
        config.components.insert("d".to_string(), "c".to_string());

        // Should validate successfully
        assert!(config.validate().is_ok());

        // Create another chain to a path: z → y → x → /path/to/file.wasm
        config
            .components
            .insert("x".to_string(), "/tmp/handler.wasm".to_string());
        config.components.insert("y".to_string(), "x".to_string());
        config.components.insert("z".to_string(), "y".to_string());

        // Should still validate
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_alias_chain_error_shows_full_path() {
        let mut config = WasmcpConfig::default();

        // Create circular chain: a → b → c → a
        config.components.insert("a".to_string(), "b".to_string());
        config.components.insert("b".to_string(), "c".to_string());
        config.components.insert("c".to_string(), "a".to_string());

        let result = config.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        let error_msg = errors.join("\n");

        // Error should show the chain
        assert!(error_msg.contains("→"));
        assert!(error_msg.contains("Circular alias dependency"));
    }

    #[test]
    fn test_profile_inheritance_chain() {
        let mut config = WasmcpConfig::default();

        // Create multi-level inheritance: prod → staging → dev → base
        config.profiles.insert(
            "base".to_string(),
            Profile {
                base: None,
                components: vec!["one".to_string()],
                output: "base.wasm".to_string(),
            },
        );
        config.profiles.insert(
            "dev".to_string(),
            Profile {
                base: Some("base".to_string()),
                components: vec!["two".to_string()],
                output: "dev.wasm".to_string(),
            },
        );
        config.profiles.insert(
            "staging".to_string(),
            Profile {
                base: Some("dev".to_string()),
                components: vec!["three".to_string()],
                output: "staging.wasm".to_string(),
            },
        );
        config.profiles.insert(
            "prod".to_string(),
            Profile {
                base: Some("staging".to_string()),
                components: vec!["four".to_string()],
                output: "prod.wasm".to_string(),
            },
        );

        // Should validate successfully
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_profile_circular_inheritance_shows_chain() {
        let mut config = WasmcpConfig::default();

        // Create circular inheritance: a → b → c → a
        config.profiles.insert(
            "a".to_string(),
            Profile {
                base: Some("b".to_string()),
                components: vec!["one".to_string()],
                output: "a.wasm".to_string(),
            },
        );
        config.profiles.insert(
            "b".to_string(),
            Profile {
                base: Some("c".to_string()),
                components: vec!["two".to_string()],
                output: "b.wasm".to_string(),
            },
        );
        config.profiles.insert(
            "c".to_string(),
            Profile {
                base: Some("a".to_string()),
                components: vec!["three".to_string()],
                output: "c.wasm".to_string(),
            },
        );

        let result = config.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        let error_msg = errors.join("\n");

        // Error should show the inheritance chain
        assert!(error_msg.contains("→"));
        assert!(error_msg.contains("Circular profile inheritance"));
    }
}
