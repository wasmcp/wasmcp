//! Component inspection utilities for WebAssembly composition
//!
//! This module provides functions to inspect WebAssembly components,
//! including checking their imports, exports, and interface details.

use anyhow::{Context, Result};
use std::path::Path;
use wit_component::DecodedWasm;
use wit_parser::{Resolve, WorldId};

/// Decode a component to extract its WIT metadata
///
/// This is a common helper to avoid repeated decoding logic across inspection functions.
fn decode_component_world(component_path: &Path) -> Result<(Resolve, WorldId)> {
    let bytes = std::fs::read(component_path)
        .with_context(|| format!("Failed to read component from {}", component_path.display()))?;

    let decoded = wit_component::decode(&bytes).with_context(|| {
        format!(
            "Failed to decode component from {}",
            component_path.display()
        )
    })?;

    match decoded {
        DecodedWasm::Component(resolve, world_id) => Ok((resolve, world_id)),
        DecodedWasm::WitPackage(_, _) => {
            anyhow::bail!(
                "Expected a component, found a WIT package at {}",
                component_path.display()
            )
        }
    }
}

/// Check what interfaces a component exports (for debugging)
pub fn check_component_exports(component_path: &Path) -> Result<Vec<String>> {
    let (resolve, world_id) = decode_component_world(component_path)?;
    let world = &resolve.worlds[world_id];
    let mut exports = Vec::new();

    for (key, _item) in &world.exports {
        if let wit_parser::WorldKey::Interface(id) = key {
            let interface = &resolve.interfaces[*id];
            if let Some(package_id) = interface.package {
                let package = &resolve.packages[package_id];
                let full_name = format!(
                    "{}:{}/{}@{}",
                    package.name.namespace,
                    package.name.name,
                    interface.name.as_ref().unwrap_or(&"unknown".to_string()),
                    package
                        .name
                        .version
                        .as_ref()
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "0.0.0".to_string())
                );
                exports.push(full_name);
            }
        }
    }

    Ok(exports)
}

/// Check what interfaces a component imports (for debugging)
pub fn check_component_imports(component_path: &Path) -> Result<Vec<String>> {
    let (resolve, world_id) = decode_component_world(component_path)?;
    let world = &resolve.worlds[world_id];
    let mut imports = Vec::new();

    for (key, _item) in &world.imports {
        if let wit_parser::WorldKey::Interface(id) = key {
            let interface = &resolve.interfaces[*id];
            if let Some(package_id) = interface.package {
                let package = &resolve.packages[package_id];
                let full_name = format!(
                    "{}:{}/{}@{}",
                    package.name.namespace,
                    package.name.name,
                    interface.name.as_ref().unwrap_or(&"unknown".to_string()),
                    package
                        .name
                        .version
                        .as_ref()
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "0.0.0".to_string())
                );
                imports.push(full_name);
            }
        }
    }

    Ok(imports)
}

/// Get detailed interface information including function signatures
pub fn get_interface_details(component_path: &Path, interface_name: &str) -> Result<String> {
    let (resolve, world_id) = decode_component_world(component_path)?;
    let world = &resolve.worlds[world_id];

    // Search both imports and exports
    for (key, _item) in world.imports.iter().chain(world.exports.iter()) {
        if let wit_parser::WorldKey::Interface(id) = key {
            let interface = &resolve.interfaces[*id];
            if let Some(package_id) = interface.package {
                let package = &resolve.packages[package_id];
                let full_name = format!(
                    "{}:{}/{}@{}",
                    package.name.namespace,
                    package.name.name,
                    interface.name.as_ref().unwrap_or(&"unknown".to_string()),
                    package
                        .name
                        .version
                        .as_ref()
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "0.0.0".to_string())
                );

                if full_name == interface_name {
                    let mut details = String::new();
                    details.push_str(&format!("Interface: {}\n", full_name));
                    details.push_str("Functions:\n");

                    for (func_name, func) in &interface.functions {
                        details.push_str(&format!("  {}(", func_name));

                        // Parameters
                        for (i, (param_name, param_type)) in func.params.iter().enumerate() {
                            if i > 0 {
                                details.push_str(", ");
                            }
                            details.push_str(&format!("{}: {:?}", param_name, param_type));
                        }

                        details.push(')');

                        // Return type
                        details.push_str(&format!(" -> {:?}", func.result));

                        details.push('\n');
                    }

                    return Ok(details);
                }
            }
        }
    }

    anyhow::bail!("Interface {} not found in component", interface_name)
}

/// Find an interface export from a component by prefix pattern
///
/// Inspects the component binary to find an export matching the given prefix.
/// For example, prefix "wasmcp:mcp-v20250618/server-handler@" will match "wasmcp:mcp-v20250618/server-handler@0.1.0".
///
/// Returns the full interface name if found.
pub fn find_component_export(component_path: &Path, prefix: &str) -> Result<String> {
    let (resolve, world_id) = decode_component_world(component_path)?;
    let world = &resolve.worlds[world_id];

    // Search exports for matching interface
    for (key, _item) in &world.exports {
        if let wit_parser::WorldKey::Interface(id) = key {
            let interface = &resolve.interfaces[*id];
            if let Some(package_id) = interface.package {
                let package = &resolve.packages[package_id];
                // Build the full interface name: namespace:package/interface@version
                let full_name = format!(
                    "{}:{}/{}@{}",
                    package.name.namespace,
                    package.name.name,
                    interface.name.as_ref().unwrap_or(&"unknown".to_string()),
                    package
                        .name
                        .version
                        .as_ref()
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "0.0.0".to_string())
                );

                if full_name.starts_with(prefix) {
                    return Ok(full_name);
                }
            }
        }
    }

    anyhow::bail!(
        "No export found matching prefix '{}' in component at {}",
        prefix,
        component_path.display()
    )
}

/// Check if a component imports a specific interface
///
/// Inspects the component binary to determine if it imports the given interface.
/// This is used to determine whether to wire service exports to this component.
pub fn component_imports_interface(component_path: &Path, interface_name: &str) -> Result<bool> {
    let (resolve, world_id) = decode_component_world(component_path)?;
    let world = &resolve.worlds[world_id];

    // Search imports for the specified interface
    for (key, _item) in &world.imports {
        if let wit_parser::WorldKey::Interface(id) = key {
            let interface = &resolve.interfaces[*id];
            if let Some(package_id) = interface.package {
                let package = &resolve.packages[package_id];
                // Build the full interface name
                let full_name = format!(
                    "{}:{}/{}@{}",
                    package.name.namespace,
                    package.name.name,
                    interface.name.as_ref().unwrap_or(&"unknown".to_string()),
                    package
                        .name
                        .version
                        .as_ref()
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "0.0.0".to_string())
                );

                if full_name == interface_name {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}
