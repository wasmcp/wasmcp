//! Runtime detection from composed components
//!
//! This module analyzes composed WebAssembly components to detect runtime
//! requirements based on package dependencies.

use anyhow::{Context, Result};
use std::path::Path;
use wit_component::DecodedWasm;

/// Runtime information detected from a component
#[derive(Debug, Clone)]
pub struct RuntimeInfo {
    /// Required runtime capabilities (e.g., "cli", "http", "keyvalue")
    pub capabilities: Vec<String>,
    /// Detected runtime type
    pub runtime_type: RuntimeType,
}

/// Detected runtime type based on package dependencies
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeType {
    /// Wasmtime runtime (uses draft2 packages)
    Wasmtime,
    /// Spin runtime (uses different package versions)
    Spin,
    /// Unknown/generic runtime
    Generic,
}

impl RuntimeInfo {
    /// Create a default runtime info (Wasmtime with basic capabilities)
    pub fn default() -> Self {
        Self {
            capabilities: vec!["cli".to_string(), "http".to_string()],
            runtime_type: RuntimeType::Wasmtime,
        }
    }
}

/// Detect runtime requirements from a composed component
///
/// Inspects the component's world to find imported packages and determine
/// what runtime it targets and what capabilities it requires.
///
/// Detection logic:
/// - `package wasmcp:sessions@X.Y.Z` → Wasmtime (draft2)
/// - `package wasmcp:sessions-d2@X.Y.Z` → Spin/other runtime
/// - WASI imports determine required capabilities
pub fn detect_runtime(component_bytes: &[u8]) -> Result<RuntimeInfo> {
    let decoded = wit_component::decode(component_bytes)
        .context("Failed to decode component for runtime detection")?;

    let (resolve, world_id) = match decoded {
        DecodedWasm::Component(resolve, world_id) => (resolve, world_id),
        DecodedWasm::WitPackage(_, _) => {
            anyhow::bail!("Expected a component, found a WIT package")
        }
    };

    let world = &resolve.worlds[world_id];
    let mut capabilities = Vec::new();
    let mut runtime_type = RuntimeType::Generic;

    // Check all imports for package patterns
    for (key, _item) in &world.imports {
        if let wit_parser::WorldKey::Interface(id) = key {
            let interface = &resolve.interfaces[*id];
            if let Some(package_id) = interface.package {
                let package = &resolve.packages[package_id];
                let namespace = &package.name.namespace;
                let name = &package.name.name;

                // Detect runtime type from session package
                if namespace == "wasmcp" {
                    if name == "sessions" {
                        runtime_type = RuntimeType::Wasmtime;
                    } else if name == "sessions-d2" {
                        runtime_type = RuntimeType::Spin;
                    }
                }

                // Detect required WASI capabilities
                if namespace == "wasi" {
                    match name.as_str() {
                        "cli" => {
                            if !capabilities.contains(&"cli".to_string()) {
                                capabilities.push("cli".to_string());
                            }
                        }
                        "http" => {
                            if !capabilities.contains(&"http".to_string()) {
                                capabilities.push("http".to_string());
                            }
                        }
                        "keyvalue" => {
                            if !capabilities.contains(&"keyvalue".to_string()) {
                                capabilities.push("keyvalue".to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Default to wasmtime if no specific runtime detected
    if runtime_type == RuntimeType::Generic {
        runtime_type = RuntimeType::Wasmtime;
    }

    // Ensure basic capabilities are present for HTTP servers
    if !capabilities.contains(&"cli".to_string()) {
        capabilities.push("cli".to_string());
    }
    if !capabilities.contains(&"http".to_string()) {
        capabilities.push("http".to_string());
    }

    Ok(RuntimeInfo {
        capabilities,
        runtime_type,
    })
}

/// Detect runtime requirements from a component file path
pub fn detect_runtime_from_file(component_path: &Path) -> Result<RuntimeInfo> {
    let bytes = std::fs::read(component_path)
        .with_context(|| format!("Failed to read component from {}", component_path.display()))?;
    detect_runtime(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_info_default() {
        let info = RuntimeInfo::default();
        assert_eq!(info.runtime_type, RuntimeType::Wasmtime);
        assert!(info.capabilities.contains(&"cli".to_string()));
        assert!(info.capabilities.contains(&"http".to_string()));
    }
}
