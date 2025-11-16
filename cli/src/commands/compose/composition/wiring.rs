//! Component wiring utilities for WebAssembly composition
//!
//! This module handles wiring components together in the composition graph,
//! including building the middleware chain and wiring transport interfaces.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use wac_graph::{CompositionGraph, NodeId, PackageId};

use super::{CompositionPackages, ServiceRegistry};
use crate::commands::compose::inspection::UnsatisfiedImports;
use crate::commands::compose::inspection::interfaces;
use crate::commands::compose::inspection::{check_component_imports, component_imports_interface};
use crate::versioning::VersionResolver;

/// Automatically wire an interface to a component if it imports it
///
/// Checks if the component imports the specified interface, and if so,
/// wires the provided export to satisfy that import.
///
/// Returns Ok(true) if wiring occurred, Ok(false) if not needed
pub fn wire_if_imports(
    graph: &mut CompositionGraph,
    component_inst: NodeId,
    component_path: &Path,
    component_name: &str,
    service_inst: NodeId,
    interface: &str,
    service_name: &str,
    verbose: bool,
) -> Result<bool> {
    if component_imports_interface(component_path, interface)? {
        if verbose {
            eprintln!(
                "   ✓ {} imports {}, wiring it",
                component_name, service_name
            );
        }

        let export = graph
            .alias_instance_export(service_inst, interface)
            .with_context(|| format!("Failed to get {} export from {}", interface, service_name))?;

        graph
            .set_instantiation_argument(component_inst, interface, export)
            .with_context(|| format!("Failed to wire {} {} import", component_name, interface))?;

        if verbose {
            eprintln!(
                "   ✓ Successfully wired {} to {}",
                service_name, component_name
            );
        }

        Ok(true)
    } else {
        if verbose {
            eprintln!(
                "   ✗ {} does NOT import {}, skipping",
                component_name, service_name
            );
        }
        Ok(false)
    }
}

/// Automatically wire all available service exports to a component based on its imports
///
/// This discovers what the component imports and automatically wires any matching
/// service exports from the registry, eliminating the need to manually wire each interface.
///
/// Returns the count of interfaces that were wired
pub fn wire_all_services(
    graph: &mut CompositionGraph,
    component_inst: NodeId,
    component_path: &Path,
    component_name: &str,
    registry: &ServiceRegistry,
    verbose: bool,
) -> Result<usize> {
    // Get all imports from the component
    let imports = check_component_imports(component_path).with_context(|| {
        format!(
            "Failed to check imports for component '{}' at {}",
            component_name,
            component_path.display()
        )
    })?;

    if verbose && !imports.is_empty() {
        eprintln!(
            "\n[AUTO-WIRE] Component '{}' imports {} interface(s):",
            component_name,
            imports.len()
        );
        for import in &imports {
            eprintln!("[AUTO-WIRE]   - {}", import);
        }
    }

    let mut wired_count = 0;

    // For each import, check if any service in the registry exports it
    for import in &imports {
        if let Some((service_name, service_info, full_interface)) = registry.find_export(import) {
            if verbose {
                eprintln!(
                    "[AUTO-WIRE] Wiring '{}' from service '{}' to component '{}'",
                    full_interface, service_name, component_name
                );
            }

            // Get the export from the service instance
            let export = graph
                .alias_instance_export(service_info.instance, full_interface)
                .with_context(|| {
                    format!(
                        "Failed to get export '{}' from service '{}'",
                        full_interface, service_name
                    )
                })?;

            // Wire it to the component's import
            graph
                .set_instantiation_argument(component_inst, full_interface, export)
                .with_context(|| {
                    format!(
                        "Failed to wire '{}' import for component '{}'",
                        full_interface, component_name
                    )
                })?;

            wired_count += 1;

            if verbose {
                eprintln!("[AUTO-WIRE]   ✓ Success");
            }
        } else if verbose {
            eprintln!(
                "[AUTO-WIRE] No service found exporting '{}' (skipping)",
                import
            );
        }
    }

    if verbose && wired_count > 0 {
        eprintln!(
            "[AUTO-WIRE] Wired {} interface(s) to component '{}'\n",
            wired_count, component_name
        );
    }

    Ok(wired_count)
}

/// Build the middleware chain by connecting components
///
/// Returns the final handler export that should be wired to transport.
/// Now uses ServiceRegistry for automatic dependency wiring.
pub fn build_middleware_chain(
    graph: &mut CompositionGraph,
    packages: &CompositionPackages,
    component_paths: &[PathBuf],
    server_handler_interface: &str,
    registry: &ServiceRegistry,
    _unsatisfied: &mut UnsatisfiedImports,
    verbose: bool,
) -> Result<NodeId> {
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
        let component_name = format!("component-{}", i);

        // Wire this component's server-handler import to the previous component's export
        graph
            .set_instantiation_argument(inst, server_handler_interface, next_handler_export)
            .with_context(|| format!("Failed to wire component-{} server-handler import", i))?;

        // Automatically wire ALL service dependencies based on component's imports
        wire_all_services(
            graph,
            inst,
            &component_paths[i],
            &component_name,
            registry,
            verbose,
        )?;

        // This component's export becomes the next input
        next_handler_export = graph
            .alias_instance_export(inst, server_handler_interface)
            .with_context(|| format!("Failed to get server-handler export from component-{}", i))?;
    }

    // Return the final handler export (from the first user component)
    // This will be wired to transport's server-handler import
    Ok(next_handler_export)
}

/// Wire the transport at the front of the chain and export its interface
///
/// Now uses ServiceRegistry for automatic dependency wiring
pub fn wire_transport(
    graph: &mut CompositionGraph,
    transport_id: PackageId,
    handler_export: NodeId,
    server_handler_interface: &str,
    transport_path: &Path,
    registry: &ServiceRegistry,
    resolver: &VersionResolver,
    verbose: bool,
) -> Result<()> {
    if verbose {
        eprintln!("\n[WIRE] ==================== WIRING TRANSPORT ====================");
    }

    // Instantiate transport
    let transport_inst = graph.instantiate(transport_id);
    if verbose {
        eprintln!("[WIRE] Instantiated transport component");
    }

    // Wire transport's server-handler import to the middleware chain
    if verbose {
        eprintln!("\n[WIRE] 1. Wiring server-handler (middleware chain)...");
        eprintln!("[WIRE]    Interface: {}", server_handler_interface);
    }
    graph
        .set_instantiation_argument(transport_inst, server_handler_interface, handler_export)
        .context("Failed to wire transport server-handler import")?;
    if verbose {
        eprintln!("[WIRE]    ✓ Success");
    }

    // Automatically wire ALL service dependencies
    if verbose {
        eprintln!("\n[WIRE] 2. Auto-wiring service dependencies...");
    }
    let wired_count = wire_all_services(
        graph,
        transport_inst,
        transport_path,
        "transport",
        registry,
        verbose,
    )?;

    if verbose {
        eprintln!(
            "[WIRE]    ✓ Auto-wired {} service interface(s)\n",
            wired_count
        );
    }

    // Export both HTTP and CLI interfaces from the transport component
    // This allows the composed component to be run with either wasmtime serve or wasmtime run
    if verbose {
        eprintln!("[WIRE] 3. Exporting transport interfaces...");
    }

    let wasi_http = interfaces::wasi_http_handler(resolver)?;
    let http_handler = graph.alias_instance_export(transport_inst, &wasi_http)?;
    graph.export(http_handler, &wasi_http)?;
    if verbose {
        eprintln!("[WIRE]    ✓ Exported HTTP handler interface");
    }

    let wasi_cli = interfaces::wasi_cli_run(resolver)?;
    let cli_run = graph.alias_instance_export(transport_inst, &wasi_cli)?;
    graph.export(cli_run, &wasi_cli)?;
    if verbose {
        eprintln!("[WIRE]    ✓ Exported CLI run interface");
    }

    if verbose {
        eprintln!("\n[WIRE] ==================== WIRING COMPLETE ====================\n");
    }

    Ok(())
}
