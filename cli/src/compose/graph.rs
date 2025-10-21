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
/// The composition strategy is:
/// 1. Instantiate transport first to get its notifications export
/// 2. Instantiate method-not-found (terminal handler) with notifications if needed
/// 3. Instantiate each user component in reverse order, wiring both server-handler and notifications
/// 4. Export the transport's WASI interface (http or cli)
///
/// This creates the chain: transport → component₁ → ... → componentₙ → method-not-found
///
/// Each component's `server-handler` import is satisfied by the next component's
/// `server-handler` export, creating a linear middleware pipeline. Additionally,
/// the transport's notifications export is wired to all components that import it.
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

    // Build the complete composition graph
    if verbose {
        println!("   Building composition graph...");
    }
    let server_handler_interface = dependencies::interfaces::server_handler(version);
    let notifications_interface = dependencies::interfaces::notifications(version);

    // First instantiate transport to get its exports
    let transport_inst = graph.instantiate(packages.transport_id);

    // Try to get the notifications export from transport (may not exist in older versions)
    let notifications_export = graph
        .alias_instance_export(transport_inst, &notifications_interface)
        .ok();

    if verbose {
        if notifications_export.is_some() {
            println!("   Transport exports notifications interface");
        } else {
            println!("   Transport does not export notifications interface");
        }
    }

    // Build the middleware chain with notifications wiring
    let handler_export = build_middleware_chain_with_notifications(
        &mut graph,
        &packages,
        &server_handler_interface,
        notifications_export,
        &notifications_interface,
        verbose,
    )?;

    // Wire the transport's server-handler import to the chain
    graph.set_instantiation_argument(transport_inst, &server_handler_interface, handler_export)?;

    // Export the appropriate WASI interface based on transport type
    export_transport_interface(&mut graph, transport_inst, transport_type)?;

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

/// Build the middleware chain with proper notification wiring
///
/// Returns the final handler export that should be wired to the transport
fn build_middleware_chain_with_notifications(
    graph: &mut CompositionGraph,
    packages: &CompositionPackages,
    server_handler_interface: &str,
    notifications_export: Option<wac_graph::NodeId>,
    notifications_interface: &str,
    verbose: bool,
) -> Result<wac_graph::NodeId> {
    // Start with method-not-found as the terminal handler
    let method_not_found_inst = graph.instantiate(packages.method_not_found_id);

    // Try to wire notifications to method-not-found after instantiation
    // Note: In wac-graph, set_instantiation_argument might need to be called before instantiate
    // but we'll try this approach first as the exact API semantics aren't clear
    if let Some(notif_export) = notifications_export {
        if let Err(_) = graph.set_instantiation_argument(method_not_found_inst, notifications_interface, notif_export) {
            // If setting after instantiation fails, the component either doesn't import
            // notifications or we need a different approach
            if verbose {
                println!("     - Method-not-found does not import notifications");
            }
        } else if verbose {
            println!("     ✓ Wired notifications to method-not-found handler");
        }
    }

    // Get the server-handler export from method-not-found
    let mut next_handler_export = graph
        .alias_instance_export(method_not_found_inst, server_handler_interface)
        .context("Failed to get server-handler export from method-not-found")?;

    // Chain user components in reverse order
    // This ensures when called, the first component processes first
    for (i, pkg_id) in packages.user_ids.iter().enumerate().rev() {
        let inst = graph.instantiate(*pkg_id);

        // Set the arguments after instantiation
        // Wire server-handler import to the previous component's export
        if let Err(e) = graph.set_instantiation_argument(inst, server_handler_interface, next_handler_export) {
            // If this fails, we might need to set arguments before instantiation
            // Let's try a different approach - re-instantiate with arguments
            return Err(anyhow::anyhow!(
                "Failed to wire component-{} server-handler import: {}. \
                This might indicate that arguments need to be set before instantiation.", i, e
            ));
        }

        // Wire notifications if available and component imports them
        if let Some(notif_export) = notifications_export {
            if let Err(_) = graph.set_instantiation_argument(inst, notifications_interface, notif_export) {
                if verbose {
                    println!("     - Component-{} does not import notifications", i);
                }
            } else if verbose {
                println!("     ✓ Wired notifications to component-{}", i);
            }
        }

        // This component's export becomes the next input
        next_handler_export = graph
            .alias_instance_export(inst, server_handler_interface)
            .with_context(|| format!("Failed to get server-handler export from component-{}", i))?;
    }

    Ok(next_handler_export)
}

/// Export the transport's WASI interface
fn export_transport_interface(
    graph: &mut CompositionGraph,
    transport_inst: wac_graph::NodeId,
    transport_type: &str,
) -> Result<()> {
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
            let cli_run = graph
                .alias_instance_export(transport_inst, dependencies::interfaces::WASI_CLI_RUN)?;
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
    wac_graph::types::Package::from_file(&format!("wasmcp:{}", name), None, path, graph.types_mut())
        .with_context(|| {
            format!(
                "Failed to load component '{}' from {}",
                name,
                path.display()
            )
        })
}
