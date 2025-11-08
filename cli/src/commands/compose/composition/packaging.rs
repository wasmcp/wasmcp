//! Package loading and registration utilities for WebAssembly composition
//!
//! This module handles loading WebAssembly components and registering them
//! as packages within the wac-graph composition system.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use wac_graph::CompositionGraph;

/// Package IDs for composition
pub struct CompositionPackages {
    pub transport_id: wac_graph::PackageId,
    pub server_io_id: wac_graph::PackageId,
    pub kv_store_id: wac_graph::PackageId,
    pub session_store_id: wac_graph::PackageId,
    pub user_ids: Vec<wac_graph::PackageId>,
    pub method_not_found_id: wac_graph::PackageId,
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
    server_io_path: &Path,
    kv_store_path: &Path,
    session_store_path: &Path,
    component_paths: &[PathBuf],
    method_not_found_path: &Path,
    verbose: bool,
) -> Result<CompositionPackages> {
    // Load packages
    let transport_pkg = load_package(graph, "transport", transport_path, verbose)?;
    let server_io_pkg = load_package(graph, "server-io", server_io_path, verbose)?;
    let kv_store_pkg = load_package(graph, "kv-store", kv_store_path, verbose)?;
    let session_store_pkg = load_package(graph, "session-store", session_store_path, verbose)?;
    let method_not_found_pkg =
        load_package(graph, "method-not-found", method_not_found_path, verbose)?;

    let mut user_packages = Vec::new();
    for (i, path) in component_paths.iter().enumerate() {
        // Use index to ensure unique names even if components have same filename
        let name = format!("component-{}", i);
        let pkg = load_package(graph, &name, path, verbose)?;
        user_packages.push(pkg);
    }

    // Register packages
    let transport_id = graph.register_package(transport_pkg)?;
    let server_io_id = graph.register_package(server_io_pkg)?;
    let kv_store_id = graph.register_package(kv_store_pkg)?;
    let session_store_id = graph.register_package(session_store_pkg)?;
    let method_not_found_id = graph.register_package(method_not_found_pkg)?;

    let mut user_ids = Vec::new();
    for pkg in user_packages {
        user_ids.push(graph.register_package(pkg)?);
    }

    Ok(CompositionPackages {
        transport_id,
        server_io_id,
        kv_store_id,
        session_store_id,
        user_ids,
        method_not_found_id,
    })
}
