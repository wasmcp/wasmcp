//! Service registry for automatic component composition
//!
//! This module provides a registry that tracks available service components
//! and their exported interfaces, enabling automatic wiring without hardcoding
//! interface names.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use wac_graph::NodeId;

use crate::commands::compose::inspection::check_component_exports;

/// A service component with its instance and available exports
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    /// The instance ID in the composition graph
    pub instance: NodeId,
    /// The component's filesystem path (for debugging)
    pub path: String,
    /// Map of interface name -> full versioned interface string
    /// e.g., "wasmcp:keyvalue/store" -> "wasmcp:keyvalue/store@0.1.0"
    pub exports: HashMap<String, String>,
}

/// Registry of available service components and their exports
#[derive(Debug, Default)]
pub struct ServiceRegistry {
    /// Map of service name -> ServiceInfo
    services: HashMap<String, ServiceInfo>,
}

impl ServiceRegistry {
    /// Create a new empty service registry
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    /// Register a service component by discovering its exports
    ///
    /// This inspects the component binary to find all exported interfaces
    /// and stores them for later automatic wiring.
    pub fn register_service(
        &mut self,
        name: impl Into<String>,
        instance: NodeId,
        component_path: &Path,
    ) -> Result<()> {
        let name = name.into();
        let path = component_path.display().to_string();

        // Discover all exports from the component
        let export_list = check_component_exports(component_path).with_context(|| {
            format!(
                "Failed to discover exports for service '{}' at {}",
                name, path
            )
        })?;

        // Build a map from interface base name to full versioned name
        // e.g., "wasmcp:keyvalue/store@0.1.0" -> key: "wasmcp:keyvalue/store", value: full string
        let mut exports = HashMap::new();
        for export in export_list {
            // Extract the base interface name (without version)
            if let Some(base) = export.rsplit_once('@').map(|(base, _version)| base) {
                exports.insert(base.to_string(), export.clone());
            } else {
                // No version suffix, use as-is
                exports.insert(export.clone(), export);
            }
        }

        self.services.insert(
            name.clone(),
            ServiceInfo {
                instance,
                path: path.clone(),
                exports: exports.clone(),
            },
        );

        Ok(())
    }

    /// Find which service exports a given interface (by prefix or exact name)
    ///
    /// Returns (service_name, service_info, full_interface_name) if found
    pub fn find_export(&self, interface_pattern: &str) -> Option<(&String, &ServiceInfo, &String)> {
        for (service_name, service_info) in &self.services {
            // Check for exact match first
            if let Some(full_name) = service_info.exports.get(interface_pattern) {
                return Some((service_name, service_info, full_name));
            }

            // Check for prefix match (allows partial interface names)
            for (base_name, full_name) in &service_info.exports {
                if base_name.starts_with(interface_pattern)
                    || full_name.starts_with(interface_pattern)
                {
                    return Some((service_name, service_info, full_name));
                }
            }
        }

        None
    }

    /// Get all exported interfaces from all services
    ///
    /// Returns a list of (service_name, interface_base_name, full_interface_name)
    pub fn all_exports(&self) -> Vec<(&String, &String, &String)> {
        let mut result = Vec::new();
        for (service_name, service_info) in &self.services {
            for (base_name, full_name) in &service_info.exports {
                result.push((service_name, base_name, full_name));
            }
        }
        result
    }

    /// Get a service by name
    pub fn get_service(&self, name: &str) -> Option<&ServiceInfo> {
        self.services.get(name)
    }

    /// Get all service names
    pub fn service_names(&self) -> impl Iterator<Item = &String> {
        self.services.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_base_name_extraction() {
        let full = "wasmcp:keyvalue/store@0.1.0";
        let base = full.rsplit_once('@').map(|(base, _)| base).unwrap();
        assert_eq!(base, "wasmcp:keyvalue/store");
    }

    #[test]
    fn test_service_registry_creation() {
        let registry = ServiceRegistry::new();
        assert_eq!(registry.service_names().count(), 0);
    }

    #[test]
    fn test_service_registry_default() {
        let registry = ServiceRegistry::default();
        assert!(registry.get_service("nonexistent").is_none());
    }
}
