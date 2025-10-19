//! WebAssembly component composition using wac-graph
//!
//! This module handles building the component composition graph that chains
//! transport → components → method-not-found into a complete MCP server.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use wac_graph::{CompositionGraph, EncodeOptions};

use super::dependencies;

/// Package IDs for composition
struct CompositionPackages {
    transport_id: wac_graph::PackageId,
    user_ids: Vec<wac_graph::PackageId>,
    method_not_found_id: wac_graph::PackageId,
}

/// Build the component composition using wac-graph
///
/// The composition strategy is simple:
/// 1. Instantiate method-not-found (terminal handler)
/// 2. Instantiate each user component in reverse order, wiring to previous
/// 3. Instantiate transport at the front, wiring to the chain
/// 4. Export the transport's WASI interface (http or cli)
///
/// This creates the chain: transport → component₁ → ... → componentₙ → method-not-found
///
/// Each component's `server-handler` import is satisfied by the next component's
/// `server-handler` export, creating a linear middleware pipeline.
pub async fn build_composition(
    transport_path: &Path,
    component_paths: &[PathBuf],
    method_not_found_path: &Path,
    transport_type: &str,
    version: &str,
    verbose: bool,
) -> Result<Vec<u8>> {
    let mut graph = CompositionGraph::new();

    // Load and register all components
    if verbose {
        println!("   Loading components...");
    }
    let packages = load_and_register_components(
        &mut graph,
        transport_path,
        component_paths,
        method_not_found_path,
    )?;

    // Build the middleware chain
    if verbose {
        println!("   Building composition graph...");
    }
    let server_handler_interface = dependencies::interfaces::server_handler(version);
    let handler_export = build_middleware_chain(&mut graph, &packages, &server_handler_interface)?;

    // Wire transport and export interface
    wire_transport(
        &mut graph,
        packages.transport_id,
        handler_export,
        transport_type,
        &server_handler_interface,
    )?;

    // Encode the composition
    if verbose {
        println!("   Encoding component...");
    }
    let bytes = graph
        .encode(EncodeOptions::default())
        .context("Failed to encode composition")?;

    Ok(bytes)
}

/// Load and register all components with the composition graph
fn load_and_register_components(
    graph: &mut CompositionGraph,
    transport_path: &Path,
    component_paths: &[PathBuf],
    method_not_found_path: &Path,
) -> Result<CompositionPackages> {
    // Load packages
    let transport_pkg = load_package(graph, "transport", transport_path)?;
    let method_not_found_pkg = load_package(graph, "method-not-found", method_not_found_path)?;

    let mut user_packages = Vec::new();
    for (i, path) in component_paths.iter().enumerate() {
        // Use index to ensure unique names even if components have same filename
        let name = format!("component-{}", i);
        let pkg = load_package(graph, &name, path)?;
        user_packages.push(pkg);
    }

    // Register packages
    let transport_id = graph.register_package(transport_pkg)?;
    let method_not_found_id = graph.register_package(method_not_found_pkg)?;

    let mut user_ids = Vec::new();
    for pkg in user_packages {
        user_ids.push(graph.register_package(pkg)?);
    }

    Ok(CompositionPackages {
        transport_id,
        user_ids,
        method_not_found_id,
    })
}

/// Build the middleware chain by connecting components
///
/// Returns the final handler export that should be wired to the transport
fn build_middleware_chain(
    graph: &mut CompositionGraph,
    packages: &CompositionPackages,
    server_handler_interface: &str,
) -> Result<wac_graph::NodeId> {
    // Start with method-not-found as the terminal handler
    let prev_inst = graph.instantiate(packages.method_not_found_id);

    // Get the server-handler export from method-not-found
    let mut next_handler_export = graph
        .alias_instance_export(prev_inst, server_handler_interface)
        .context("Failed to get server-handler export from method-not-found")?;

    // Chain user components in reverse order
    // This ensures when called, the first component processes first
    for (i, pkg_id) in packages.user_ids.iter().enumerate().rev() {
        let inst = graph.instantiate(*pkg_id);

        // Wire this component's server-handler import to the previous component's export
        graph
            .set_instantiation_argument(inst, server_handler_interface, next_handler_export)
            .with_context(|| format!("Failed to wire component-{} import", i))?;

        // This component's export becomes the next input
        next_handler_export = graph
            .alias_instance_export(inst, server_handler_interface)
            .with_context(|| format!(
                "Failed to get server-handler export from component-{}",
                i
            ))?;
    }

    Ok(next_handler_export)
}

/// Wire the transport at the front of the chain and export its interface
fn wire_transport(
    graph: &mut CompositionGraph,
    transport_id: wac_graph::PackageId,
    handler_export: wac_graph::NodeId,
    transport_type: &str,
    server_handler_interface: &str,
) -> Result<()> {
    // Wire transport at the front of the chain
    let transport_inst = graph.instantiate(transport_id);
    graph.set_instantiation_argument(
        transport_inst,
        server_handler_interface,
        handler_export,
    )?;

    // Export the appropriate WASI interface based on transport type
    match transport_type {
        "http" => {
            let http_handler = graph.alias_instance_export(
                transport_inst,
                dependencies::interfaces::WASI_HTTP_HANDLER,
            )?;
            graph.export(http_handler, dependencies::interfaces::WASI_HTTP_HANDLER)?;
        }
        "stdio" => {
            let cli_run = graph.alias_instance_export(
                transport_inst,
                dependencies::interfaces::WASI_CLI_RUN,
            )?;
            graph.export(cli_run, dependencies::interfaces::WASI_CLI_RUN)?;
        }
        _ => anyhow::bail!("unsupported transport type: '{}'", transport_type),
    }

    Ok(())
}

/// Load a WebAssembly component as a package in the composition graph
///
/// This reads the component file and registers it with wac-graph's type system.
pub fn load_package(
    graph: &mut CompositionGraph,
    name: &str,
    path: &Path,
) -> Result<wac_graph::types::Package> {
    wac_graph::types::Package::from_file(
        &format!("wasmcp:{}", name),
        None,
        path,
        graph.types_mut(),
    )
    .with_context(|| format!(
        "Failed to load component '{}' from {}",
        name,
        path.display()
    ))
}
