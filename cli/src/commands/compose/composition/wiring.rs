//! Component wiring utilities for WebAssembly composition
//!
//! This module handles wiring components together in the composition graph,
//! including building the middleware chain and wiring transport interfaces.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use wac_graph::{CompositionGraph, NodeId, PackageId};

use super::CompositionPackages;
use crate::commands::compose::inspection::UnsatisfiedImports;
use crate::commands::compose::inspection::interfaces;
use crate::commands::compose::inspection::{component_imports_interface, get_interface_details};
use crate::versioning::VersionResolver;

/// Build the middleware chain by connecting components
///
/// Returns the final handler export that should be wired to transport
///
/// TODO: Refactor to reduce argument count (10/7). Consider grouping into a
/// MiddlewareChainConfig struct (instances, interfaces, paths, tracker).
#[allow(clippy::too_many_arguments)]
pub fn build_middleware_chain(
    graph: &mut CompositionGraph,
    packages: &CompositionPackages,
    server_io_inst: NodeId,
    session_store_inst: NodeId,
    component_paths: &[PathBuf],
    server_handler_interface: &str,
    server_io_interface: &str,
    sessions_interface: &str,
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

        // Wire this component's server-handler import to the previous component's export
        graph
            .set_instantiation_argument(inst, server_handler_interface, next_handler_export)
            .with_context(|| format!("Failed to wire component-{} server-handler import", i))?;

        // Check if this component imports server-io and wire it if so
        if verbose {
            eprintln!(
                "[MIDDLEWARE] Checking if component-{} imports server-io...",
                i
            );
        }
        if component_imports_interface(&component_paths[i], server_io_interface)? {
            if verbose {
                eprintln!(
                    "[MIDDLEWARE]   ✓ component-{} DOES import server-io, wiring it",
                    i
                );
            }
            let server_io_export = graph
                .alias_instance_export(server_io_inst, server_io_interface)
                .context("Failed to get server-io export from server-io")?;

            graph
                .set_instantiation_argument(inst, server_io_interface, server_io_export)
                .with_context(|| format!("Failed to wire component-{} server-io import", i))?;
            if verbose {
                eprintln!(
                    "[MIDDLEWARE]   ✓ Successfully wired server-io to component-{}",
                    i
                );
            }
        } else if verbose {
            eprintln!(
                "[MIDDLEWARE]   ✗ component-{} does NOT import server-io, skipping",
                i
            );
        }

        // Check if this component imports sessions and wire it if so
        if verbose {
            eprintln!(
                "[MIDDLEWARE] Checking if component-{} imports sessions...",
                i
            );
        }
        if component_imports_interface(&component_paths[i], sessions_interface)? {
            if verbose {
                eprintln!(
                    "[MIDDLEWARE]   ✓ component-{} DOES import sessions, wiring it",
                    i
                );
            }
            let sessions_export = graph
                .alias_instance_export(session_store_inst, sessions_interface)
                .context("Failed to get sessions export from session-store")?;

            graph
                .set_instantiation_argument(inst, sessions_interface, sessions_export)
                .with_context(|| format!("Failed to wire component-{} sessions import", i))?;
            if verbose {
                eprintln!(
                    "[MIDDLEWARE]   ✓ Successfully wired sessions to component-{}",
                    i
                );
            }
        } else if verbose {
            eprintln!(
                "[MIDDLEWARE]   ✗ component-{} does NOT import sessions, skipping",
                i
            );
        }

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
#[allow(clippy::too_many_arguments)]
pub fn wire_transport(
    graph: &mut CompositionGraph,
    transport_id: PackageId,
    server_io_inst: NodeId,
    session_store_inst: NodeId,
    handler_export: NodeId,
    server_handler_interface: &str,
    server_io_interface: &str,
    sessions_interface: &str,
    session_manager_interface: &str,
    transport_path: &Path,
    server_io_path: &Path,
    _session_store_path: &Path,
    resolver: &VersionResolver,
    verbose: bool,
) -> Result<()> {
    if verbose {
        eprintln!("\n[WIRE] ==================== WIRING ANALYSIS ====================");
    }

    // Instantiate transport
    let transport_inst = graph.instantiate(transport_id);
    if verbose {
        eprintln!("[WIRE] Instantiated transport");
        eprintln!("\n[WIRE] --- Interface Discovery Results ---");
        eprintln!(
            "[WIRE] Discovered server-handler: {}",
            server_handler_interface
        );
        eprintln!("[WIRE] Discovered server-io: {}", server_io_interface);
        eprintln!("[WIRE] Discovered sessions: {}", sessions_interface);
        eprintln!(
            "[WIRE] Discovered session-manager: {}",
            session_manager_interface
        );
        eprintln!("\n[WIRE] --- Wiring Transport ---");
        eprintln!("[WIRE] 1. Wiring server-handler...");
        eprintln!("[WIRE]    Interface: {}", server_handler_interface);
    }

    // Wire transport's server-handler import to the middleware chain
    match graph.set_instantiation_argument(transport_inst, server_handler_interface, handler_export)
    {
        Ok(_) => {
            if verbose {
                eprintln!("[WIRE]    ✓ Success");
            }
        }
        Err(e) => {
            return Err(e).context("Failed to wire transport server-handler import");
        }
    }

    // Wire transport's server-io import to the server-io service
    if verbose {
        eprintln!("\n[WIRE] 2. Wiring server-io...");
        eprintln!("[WIRE]    Interface: {}", server_io_interface);
        eprintln!("\n[WIRE]    === SIGNATURE COMPARISON ===");
        eprintln!("[WIRE]    What transport IMPORTS:");
        match get_interface_details(transport_path, server_io_interface) {
            Ok(details) => {
                for line in details.lines() {
                    eprintln!("[WIRE]      {}", line);
                }
            }
            Err(e) => eprintln!("[WIRE]      ERROR: {}", e),
        }

        eprintln!("\n[WIRE]    What server-io EXPORTS:");
        match get_interface_details(server_io_path, server_io_interface) {
            Ok(details) => {
                for line in details.lines() {
                    eprintln!("[WIRE]      {}", line);
                }
            }
            Err(e) => eprintln!("[WIRE]      ERROR: {}", e),
        }
        eprintln!("[WIRE]    === END SIGNATURE COMPARISON ===\n");
        eprintln!("[WIRE]    Attempting to get export from server-io instance...");
    }

    let server_io_export = match graph.alias_instance_export(server_io_inst, server_io_interface) {
        Ok(export) => {
            if verbose {
                eprintln!("[WIRE]    ✓ Got export from server-io");
            }
            export
        }
        Err(e) => {
            return Err(e).with_context(|| format!("Failed to get server-io export for interface '{}'. Server-io component may not export this exact interface name.", server_io_interface));
        }
    };

    if verbose {
        eprintln!("[WIRE]    Attempting to wire transport import to server-io export...");
    }
    match graph.set_instantiation_argument(transport_inst, server_io_interface, server_io_export) {
        Ok(_) => {
            if verbose {
                eprintln!("[WIRE]    ✓ Success");
            }
        }
        Err(e) => {
            return Err(e).with_context(|| {
                format!(
                    "Failed to wire transport server-io import for interface '{}'",
                    server_io_interface
                )
            });
        }
    }

    // Wire transport's sessions import to the session-store service
    if verbose {
        eprintln!("\n[WIRE] 3. Wiring sessions...");
        eprintln!("[WIRE]    Interface: {}", sessions_interface);
    }
    let sessions_export = graph
        .alias_instance_export(session_store_inst, sessions_interface)
        .context("Failed to get sessions export")?;
    match graph.set_instantiation_argument(transport_inst, sessions_interface, sessions_export) {
        Ok(_) => {
            if verbose {
                eprintln!("[WIRE]    ✓ Success");
            }
        }
        Err(e) => {
            if verbose {
                eprintln!("[WIRE]    ✗ FAILED: {:?}", e);
            }
            return Err(e).context("Failed to wire transport sessions import");
        }
    }

    // Wire transport's session-manager import to the session-store service
    if verbose {
        eprintln!("\n[WIRE] 4. Wiring session-manager...");
        eprintln!("[WIRE]    Interface: {}", session_manager_interface);
    }
    let session_manager_export = graph
        .alias_instance_export(session_store_inst, session_manager_interface)
        .context("Failed to get session-manager export")?;
    match graph.set_instantiation_argument(
        transport_inst,
        session_manager_interface,
        session_manager_export,
    ) {
        Ok(_) => {
            if verbose {
                eprintln!("[WIRE]    ✓ Success");
            }
        }
        Err(e) => {
            if verbose {
                eprintln!("[WIRE]    ✗ FAILED: {:?}", e);
            }
            return Err(e).context("Failed to wire transport session-manager import");
        }
    }

    if verbose {
        eprintln!("\n[WIRE] --- Exporting Transport Interfaces ---");
    }
    // Export both HTTP and CLI interfaces from the transport component
    // This allows the composed component to be run with either wasmtime serve or wasmtime run

    if verbose {
        eprintln!("[WIRE] Exporting HTTP handler interface");
    }
    let wasi_http = interfaces::wasi_http_handler(resolver)?;
    let http_handler = graph.alias_instance_export(transport_inst, &wasi_http)?;
    graph.export(http_handler, &wasi_http)?;

    if verbose {
        eprintln!("[WIRE] Exporting CLI run interface");
    }
    let wasi_cli = interfaces::wasi_cli_run(resolver)?;
    let cli_run = graph.alias_instance_export(transport_inst, &wasi_cli)?;
    graph.export(cli_run, &wasi_cli)?;

    if verbose {
        eprintln!("[WIRE] Both interfaces exported successfully");
        eprintln!("[WIRE] ==================== WIRING COMPLETE ====================\n");
    }

    Ok(())
}
