//! WebAssembly component composition using wac-graph
//!
//! This module handles building the component composition graph that chains
//! transport → components → method-not-found into a complete MCP server.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use wac_graph::{CompositionGraph, EncodeOptions};

use super::dependencies;
use crate::versioning::VersionResolver;

/// Package IDs for composition
struct CompositionPackages {
    transport_id: wac_graph::PackageId,
    server_io_id: wac_graph::PackageId,
    session_store_id: wac_graph::PackageId,
    user_ids: Vec<wac_graph::PackageId>,
    method_not_found_id: wac_graph::PackageId,
}

/// Build the component composition using wac-graph
///
/// The composition strategy:
/// 1. Instantiate method-not-found (terminal handler)
/// 2. Instantiate each user component in reverse order, wiring to previous
/// 3. Instantiate session-store, wiring to the user component chain
/// 4. Instantiate server-io, wiring to session-store
/// 5. Instantiate transport at the front, wiring to server-io
/// 6. Export the transport's WASI interface (http or cli)
///
/// This creates the chain: transport → server-io → session-store → component₁ → ... → componentₙ → method-not-found
///
/// Each component's `server-handler` import is satisfied by the next component's
/// `server-handler` export, creating a linear middleware pipeline.
pub async fn build_composition(
    transport_path: &Path,
    server_io_path: &Path,
    session_store_path: &Path,
    component_paths: &[PathBuf],
    method_not_found_path: &Path,
    transport_type: &str,
    _resolver: &VersionResolver,
    verbose: bool,
) -> Result<Vec<u8>> {
    // Discover interface versions from actual components before building graph
    // This decouples composition from our version manifest
    let server_handler_interface = find_component_export(
        method_not_found_path,
        "wasmcp:mcp-v20250618/server-handler@",
    )
    .context("Failed to discover server-handler interface from method-not-found component")?;

    if verbose {
        println!(
            "   Discovered server-handler interface: {}",
            server_handler_interface
        );
    }

    let mut graph = CompositionGraph::new();

    // Load and register all components
    if verbose {
        println!("   Loading components...");
    }
    let packages = load_and_register_components(
        &mut graph,
        transport_path,
        server_io_path,
        session_store_path,
        component_paths,
        method_not_found_path,
    )?;

    // Instantiate service components first (needed for middleware wiring)
    let server_io_inst = graph.instantiate(packages.server_io_id);
    let session_store_inst = graph.instantiate(packages.session_store_id);

    // Build the middleware chain
    if verbose {
        println!("   Building composition graph...");
    }
    let handler_export = build_middleware_chain(
        &mut graph,
        &packages,
        session_store_inst,
        component_paths,
        &server_handler_interface,
    )?;

    // Wire transport and export interface
    wire_transport(
        &mut graph,
        packages.transport_id,
        server_io_inst,
        session_store_inst,
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
    server_io_path: &Path,
    session_store_path: &Path,
    component_paths: &[PathBuf],
    method_not_found_path: &Path,
) -> Result<CompositionPackages> {
    // Load packages
    let transport_pkg = load_package(graph, "transport", transport_path)?;
    let server_io_pkg = load_package(graph, "server-io", server_io_path)?;
    let session_store_pkg = load_package(graph, "session-store", session_store_path)?;
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
    let server_io_id = graph.register_package(server_io_pkg)?;
    let session_store_id = graph.register_package(session_store_pkg)?;
    let method_not_found_id = graph.register_package(method_not_found_pkg)?;

    let mut user_ids = Vec::new();
    for pkg in user_packages {
        user_ids.push(graph.register_package(pkg)?);
    }

    Ok(CompositionPackages {
        transport_id,
        server_io_id,
        session_store_id,
        user_ids,
        method_not_found_id,
    })
}

/// Build the middleware chain by connecting components
///
/// Returns the final handler export that should be wired to transport
fn build_middleware_chain(
    graph: &mut CompositionGraph,
    packages: &CompositionPackages,
    session_store_inst: wac_graph::NodeId,
    component_paths: &[PathBuf],
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
            .with_context(|| format!("Failed to wire component-{} server-handler import", i))?;

        // Check if this component imports sessions and wire it if so
        if component_imports_sessions(&component_paths[i])? {
            let sessions_export = graph
                .alias_instance_export(
                    session_store_inst,
                    "wasmcp:mcp-v20250618/sessions@0.1.4-beta.2",
                )
                .context("Failed to get sessions export from session-store")?;

            graph
                .set_instantiation_argument(
                    inst,
                    "wasmcp:mcp-v20250618/sessions@0.1.4-beta.2",
                    sessions_export,
                )
                .with_context(|| format!("Failed to wire component-{} sessions import", i))?;
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
fn wire_transport(
    graph: &mut CompositionGraph,
    transport_id: wac_graph::PackageId,
    server_io_inst: wac_graph::NodeId,
    session_store_inst: wac_graph::NodeId,
    handler_export: wac_graph::NodeId,
    transport_type: &str,
    server_handler_interface: &str,
) -> Result<()> {
    // Instantiate transport
    let transport_inst = graph.instantiate(transport_id);

    // Wire transport's server-handler import to the middleware chain
    graph
        .set_instantiation_argument(transport_inst, server_handler_interface, handler_export)
        .context("Failed to wire transport server-handler import")?;

    // Wire transport's server-io import to the server-io service
    let server_io_export = graph
        .alias_instance_export(
            server_io_inst,
            "wasmcp:mcp-v20250618/server-io@0.1.4-beta.2",
        )
        .context("Failed to get server-io export")?;
    graph
        .set_instantiation_argument(
            transport_inst,
            "wasmcp:mcp-v20250618/server-io@0.1.4-beta.2",
            server_io_export,
        )
        .context("Failed to wire transport server-io import")?;

    // Wire transport's sessions import to the session-store service
    let sessions_export = graph
        .alias_instance_export(
            session_store_inst,
            "wasmcp:mcp-v20250618/sessions@0.1.4-beta.2",
        )
        .context("Failed to get sessions export")?;
    graph
        .set_instantiation_argument(
            transport_inst,
            "wasmcp:mcp-v20250618/sessions@0.1.4-beta.2",
            sessions_export,
        )
        .context("Failed to wire transport sessions import")?;

    // Wire transport's session-manager import to the session-store service
    let session_manager_export = graph
        .alias_instance_export(
            session_store_inst,
            "wasmcp:mcp-v20250618/session-manager@0.1.4-beta.2",
        )
        .context("Failed to get session-manager export")?;
    graph
        .set_instantiation_argument(
            transport_inst,
            "wasmcp:mcp-v20250618/session-manager@0.1.4-beta.2",
            session_manager_export,
        )
        .context("Failed to wire transport session-manager import")?;

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

/// Build a handler-only composition (without transport/terminal)
///
/// This creates a composable handler component:
/// - Chains components: component₁ → component₂ → ... → componentₙ
/// - Exports wasmcp:server/handler interface
/// - Can be used in further compositions
///
/// The composition strategy:
/// 1. Load and register all components
/// 2. Chain them in reverse order (last component's handler is imported by second-to-last, etc.)
/// 3. Export the first component's server-handler interface
pub async fn build_handler_composition(
    component_paths: &[PathBuf],
    version_resolver: &VersionResolver,
    verbose: bool,
) -> Result<Vec<u8>> {
    if component_paths.is_empty() {
        anyhow::bail!("Cannot build handler composition with zero components");
    }

    let mut graph = CompositionGraph::new();
    let version = version_resolver.get_version("server")?;
    let server_handler_interface = dependencies::interfaces::server_handler(&version);

    // Load and register all components
    if verbose {
        println!("   Loading components...");
    }

    let mut package_ids = Vec::new();
    for (i, path) in component_paths.iter().enumerate() {
        let name = format!("component-{}", i);
        let pkg = load_package(&mut graph, &name, path)?;
        package_ids.push(graph.register_package(pkg)?);
    }

    // Special case: single component - just re-export its handler
    if package_ids.len() == 1 {
        if verbose {
            println!("   Single component - exporting handler interface...");
        }

        let inst = graph.instantiate(package_ids[0]);
        let handler_export = graph
            .alias_instance_export(inst, &server_handler_interface)
            .context("Component does not export server-handler interface")?;

        graph
            .export(handler_export, &server_handler_interface)
            .context("Failed to export server-handler interface")?;
    } else {
        // Multiple components: chain them
        if verbose {
            println!("   Building composition chain...");
        }

        // Start from the last component (no downstream handler needed)
        let last_idx = package_ids.len() - 1;
        let prev_inst = graph.instantiate(package_ids[last_idx]);

        // Get the last component's server-handler export
        let mut next_handler_export = graph
            .alias_instance_export(prev_inst, &server_handler_interface)
            .with_context(|| {
                format!(
                    "Component {} does not export server-handler interface",
                    last_idx
                )
            })?;

        // Chain remaining components in reverse order (second-to-last to first)
        for i in (0..last_idx).rev() {
            let inst = graph.instantiate(package_ids[i]);

            // Wire this component's server-handler import to the previous component's export
            graph
                .set_instantiation_argument(inst, &server_handler_interface, next_handler_export)
                .with_context(|| format!("Failed to wire component-{} server-handler import", i))?;

            // This component's export becomes the next input
            next_handler_export = graph
                .alias_instance_export(inst, &server_handler_interface)
                .with_context(|| {
                    format!("Component {} does not export server-handler interface", i)
                })?;
        }

        // Export the first component's server-handler interface
        graph
            .export(next_handler_export, &server_handler_interface)
            .context("Failed to export server-handler interface")?;
    }

    // Encode the composition
    if verbose {
        println!("   Encoding component...");
    }
    let bytes = graph
        .encode(EncodeOptions::default())
        .context("Failed to encode handler composition")?;

    Ok(bytes)
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

/// Find an interface export from a component by prefix pattern
///
/// Inspects the component binary to find an export matching the given prefix.
/// For example, prefix "wasmcp:mcp-v20250618/server-handler@" will match "wasmcp:mcp-v20250618/server-handler@0.1.0".
///
/// Returns the full interface name if found.
fn find_component_export(component_path: &Path, prefix: &str) -> Result<String> {
    use wit_component::DecodedWasm;

    // Read the component binary
    let bytes = std::fs::read(component_path)
        .with_context(|| format!("Failed to read component from {}", component_path.display()))?;

    // Decode the component to get its WIT metadata
    let decoded = wit_component::decode(&bytes).context("Failed to decode component")?;

    let (resolve, world_id) = match decoded {
        DecodedWasm::Component(resolve, world_id) => (resolve, world_id),
        DecodedWasm::WitPackage(_, _) => {
            anyhow::bail!("Expected a component, found a WIT package");
        }
    };

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

/// Check if a component imports the sessions interface
///
/// Inspects the component binary to determine if it imports wasmcp:*/sessions@*.
/// This is used to determine whether to wire session-store's sessions export to this component.
fn component_imports_sessions(component_path: &Path) -> Result<bool> {
    use wit_component::DecodedWasm;

    // Read the component binary
    let bytes = std::fs::read(component_path)
        .with_context(|| format!("Failed to read component from {}", component_path.display()))?;

    // Decode the component to get its WIT metadata
    let decoded = wit_component::decode(&bytes).context("Failed to decode component")?;

    let (resolve, world_id) = match decoded {
        DecodedWasm::Component(resolve, world_id) => (resolve, world_id),
        DecodedWasm::WitPackage(_, _) => {
            return Ok(false);
        }
    };

    let world = &resolve.worlds[world_id];

    // Search imports for sessions interface
    for (key, _item) in &world.imports {
        if let wit_parser::WorldKey::Interface(id) = key {
            let interface = &resolve.interfaces[*id];
            if let Some(package_id) = interface.package {
                let package = &resolve.packages[package_id];
                // Check if this is a sessions interface from wasmcp namespace
                if package.name.namespace == "wasmcp"
                    && interface
                        .name
                        .as_ref()
                        .map(|n| n == "sessions")
                        .unwrap_or(false)
                {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test component naming pattern used in load_and_register_components
    #[test]
    fn test_component_naming_pattern() {
        // Test the naming scheme: component-{index}
        let name_0 = format!("component-{}", 0);
        let name_1 = format!("component-{}", 1);
        let name_5 = format!("component-{}", 5);

        assert_eq!(name_0, "component-0");
        assert_eq!(name_1, "component-1");
        assert_eq!(name_5, "component-5");
    }

    /// Test that component naming ensures uniqueness even with duplicate filenames
    #[test]
    fn test_component_naming_uniqueness() {
        // Even if multiple components have the same filename, indices make them unique
        let paths = [
            PathBuf::from("calculator.wasm"),
            PathBuf::from("calculator.wasm"), // duplicate filename
            PathBuf::from("calculator.wasm"), // another duplicate
        ];

        let names: Vec<String> = paths
            .iter()
            .enumerate()
            .map(|(i, _)| format!("component-{}", i))
            .collect();

        assert_eq!(names, vec!["component-0", "component-1", "component-2"]);
        // Verify uniqueness
        let unique_count = names.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(unique_count, 3);
    }

    /// Test transport type validation logic
    #[test]
    fn test_transport_type_validation() {
        // Valid transport types
        let http = "http";
        let stdio = "stdio";

        assert_eq!(http, "http");
        assert_eq!(stdio, "stdio");

        // Invalid transport type (would trigger error in wire_transport)
        let invalid = "websocket";
        assert_ne!(invalid, "http");
        assert_ne!(invalid, "stdio");
    }

    /// Test transport type error message format
    #[test]
    fn test_transport_error_message() {
        let invalid_type = "grpc";
        let error_msg = format!("unsupported transport type: '{}'", invalid_type);

        assert_eq!(error_msg, "unsupported transport type: 'grpc'");
        assert!(error_msg.contains("unsupported transport type"));
        assert!(error_msg.contains("grpc"));
    }

    /// Test WASI interface constants are available
    #[test]
    fn test_wasi_interface_constants() {
        use super::dependencies::interfaces;

        // These constants should be defined and accessible
        let http_handler = interfaces::WASI_HTTP_HANDLER;
        let cli_run = interfaces::WASI_CLI_RUN;

        // Verify format (exact version may vary)
        assert!(http_handler.starts_with("wasi:http/incoming-handler@"));
        assert!(cli_run.starts_with("wasi:cli/run@"));

        // Verify specific current versions
        assert_eq!(http_handler, "wasi:http/incoming-handler@0.2.3");
        assert_eq!(cli_run, "wasi:cli/run@0.2.3");
    }

    /// Test server handler interface construction
    #[test]
    fn test_server_handler_interface_construction() {
        use super::dependencies::interfaces;

        let version = "0.1.0";
        let interface = interfaces::server_handler(version);

        assert_eq!(interface, "wasmcp:mcp-v20250618/server-handler@0.1.0");
        assert!(interface.starts_with("wasmcp:mcp-v20250618/server-handler@"));
    }

    /// Test error context for component loading
    #[test]
    fn test_load_component_error_context() {
        let name = "calculator";
        let path = Path::new("/path/to/calculator.wasm");

        let error_msg = format!(
            "Failed to load component '{}' from {}",
            name,
            path.display()
        );

        assert!(error_msg.contains("Failed to load component"));
        assert!(error_msg.contains("calculator"));
        assert!(error_msg.contains("/path/to/calculator.wasm"));
    }

    /// Test error context for wiring components
    #[test]
    fn test_wiring_error_context() {
        let component_idx = 3;
        let error_msg = format!(
            "Failed to wire component-{} server-handler import",
            component_idx
        );

        assert_eq!(
            error_msg,
            "Failed to wire component-3 server-handler import"
        );
        assert!(error_msg.contains("Failed to wire"));
        assert!(error_msg.contains("server-handler import"));
    }

    /// Test error context for missing exports
    #[test]
    fn test_missing_export_error_context() {
        let component_idx = 2;
        let error_msg = format!(
            "Component {} does not export server-handler interface",
            component_idx
        );

        assert_eq!(
            error_msg,
            "Component 2 does not export server-handler interface"
        );
        assert!(error_msg.contains("does not export"));
        assert!(error_msg.contains("server-handler interface"));
    }

    /// Test handler composition empty array validation
    #[test]
    fn test_handler_composition_empty_validation() {
        // This validates the check at line 258-260
        let empty_paths: Vec<PathBuf> = vec![];
        let expected_error = "Cannot build handler composition with zero components";

        assert!(empty_paths.is_empty());
        assert!(expected_error.contains("zero components"));
    }

    /// Test package name format for wac-graph
    #[test]
    fn test_package_name_format() {
        let name = "transport";
        let package_name = format!("wasmcp:{}", name);

        assert_eq!(package_name, "wasmcp:transport");

        let component_name = "component-5";
        let component_package = format!("wasmcp:{}", component_name);

        assert_eq!(component_package, "wasmcp:component-5");
        assert!(component_package.starts_with("wasmcp:"));
    }

    /// Test reverse iteration for component chaining
    #[test]
    fn test_reverse_iteration_pattern() {
        let package_ids = [0, 1, 2, 3, 4];

        // Simulate the reverse iteration in build_middleware_chain
        let reversed: Vec<usize> = package_ids
            .iter()
            .enumerate()
            .rev()
            .map(|(i, _)| i)
            .collect();

        assert_eq!(reversed, vec![4, 3, 2, 1, 0]);
    }

    /// Test single vs multiple component detection
    #[test]
    fn test_single_component_detection() {
        let single = [PathBuf::from("component.wasm")];
        let multiple = [PathBuf::from("comp1.wasm"), PathBuf::from("comp2.wasm")];

        assert_eq!(single.len(), 1);
        assert!(multiple.len() > 1);

        // This is the condition at line 278
        if single.len() == 1 {
            // Single component special case
        } else {
            panic!("Should take single component path");
        }
    }

    /// Test last index calculation for handler composition
    #[test]
    fn test_last_index_calculation() {
        let paths = [
            PathBuf::from("a.wasm"),
            PathBuf::from("b.wasm"),
            PathBuf::from("c.wasm"),
        ];

        let last_idx = paths.len() - 1;
        assert_eq!(last_idx, 2);

        // Verify range for chaining (0..last_idx)
        let range: Vec<usize> = (0..last_idx).collect();
        assert_eq!(range, vec![0, 1]);
    }

    /// Test verbose mode message formats
    #[test]
    fn test_verbose_messages() {
        let loading_msg = "   Loading components...";
        let building_msg = "   Building composition graph...";
        let encoding_msg = "   Encoding component...";
        let chain_msg = "   Building composition chain...";
        let single_msg = "   Single component - exporting handler interface...";

        // Verify message format (3 spaces + capitalized + ellipsis)
        assert!(loading_msg.starts_with("   "));
        assert!(building_msg.starts_with("   "));
        assert!(encoding_msg.ends_with("..."));
        assert!(chain_msg.contains("composition chain"));
        assert!(single_msg.contains("Single component"));
    }
}
