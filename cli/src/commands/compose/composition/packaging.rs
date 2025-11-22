//! Package loading and registration utilities for WebAssembly composition
//!
//! This module handles loading WebAssembly components and registering them
//! as packages within the wac-graph composition system.

use crate::commands::compose::inspection::interfaces::ComponentType;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use wac_graph::CompositionGraph;

use std::collections::HashMap;

/// Package IDs for composition
pub struct CompositionPackages {
    /// Transport component (structural, required)
    pub transport_id: wac_graph::PackageId,
    /// Terminal handler (structural, required)
    pub method_not_found_id: wac_graph::PackageId,
    /// User components
    pub user_ids: Vec<wac_graph::PackageId>,
    /// Dynamic service components (ServiceRegistry)
    pub service_ids: HashMap<String, wac_graph::PackageId>,
}

/// Load a WebAssembly component as a package in the composition graph
///
/// This reads the component file and registers it with wac-graph's type system.
pub fn load_package(
    graph: &mut CompositionGraph,
    name: &str,
    path: &Path,
    verbose: bool,
) -> Result<wac_graph::types::Package> {
    if verbose {
        eprintln!(
            "[LOAD] Loading component '{}' from {}",
            name,
            path.display()
        );
    }
    let package = wac_graph::types::Package::from_file(
        &format!("wasmcp:{}", name),
        None,
        path,
        graph.types_mut(),
    )
    .with_context(|| {
        format!(
            "Failed to load component '{}' from {}",
            name,
            path.display()
        )
    })?;

    if verbose {
        eprintln!("[LOAD]   Component '{}' loaded successfully", name);
    }

    Ok(package)
}

/// Load and register all components with the composition graph
pub fn load_and_register_components(
    graph: &mut CompositionGraph,
    transport_path: &Path,
    service_paths: &HashMap<String, PathBuf>,
    component_paths: &[PathBuf],
    method_not_found_path: &Path,
    verbose: bool,
) -> Result<CompositionPackages> {
    // Load structural components
    let transport_pkg = load_package(
        graph,
        ComponentType::HttpTransport.name(),
        transport_path,
        verbose,
    )?;
    let method_not_found_pkg = load_package(
        graph,
        ComponentType::MethodNotFound.name(),
        method_not_found_path,
        verbose,
    )?;

    // Load service components dynamically
    let mut service_packages = HashMap::new();
    for (service_name, service_path) in service_paths {
        let pkg = load_package(graph, service_name, service_path, verbose)?;
        service_packages.insert(service_name.clone(), pkg);
    }

    // Load user components
    let mut user_packages = Vec::new();
    for (i, path) in component_paths.iter().enumerate() {
        // Use index to ensure unique names even if components have same filename
        let name = format!("component-{}", i);
        let pkg = load_package(graph, &name, path, verbose)?;
        user_packages.push(pkg);
    }

    // Register structural packages
    let transport_id = graph.register_package(transport_pkg)?;
    let method_not_found_id = graph.register_package(method_not_found_pkg)?;

    // Register service packages
    let mut service_ids = HashMap::new();
    for (service_name, pkg) in service_packages {
        let id = graph.register_package(pkg)?;
        service_ids.insert(service_name, id);
    }

    // Register user packages
    let mut user_ids = Vec::new();
    for pkg in user_packages {
        user_ids.push(graph.register_package(pkg)?);
    }

    Ok(CompositionPackages {
        transport_id,
        method_not_found_id,
        user_ids,
        service_ids,
    })
}
