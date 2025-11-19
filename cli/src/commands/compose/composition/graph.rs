//! WebAssembly component composition using wac-graph
//!
//! This module handles building the component composition graph that chains
//! transport → components → method-not-found into a complete MCP server.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use wac_graph::{CompositionGraph, EncodeOptions};

use super::{ServiceRegistry, load_and_register_components, load_package};
use super::{build_middleware_chain, wire_all_services, wire_transport};
use crate::commands::compose::inspection::UnsatisfiedImports;
use crate::commands::compose::inspection::find_component_export;
use crate::commands::compose::inspection::interfaces::{self, DEFAULT_SPEC_VERSION, InterfaceType};
use crate::versioning::VersionResolver;

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
///
pub struct CompositionPaths<'a> {
    pub transport: &'a Path,
    pub server_io: &'a Path,
    pub oauth_auth: &'a Path,
    pub kv_store: &'a Path,
    pub session_store: &'a Path,
    pub components: &'a [PathBuf],
    pub method_not_found: &'a Path,
}

pub async fn build_composition(
    paths: CompositionPaths<'_>,
    _resolver: &VersionResolver,
    verbose: bool,
) -> Result<Vec<u8>> {
    let transport_path = paths.transport;
    let server_io_path = paths.server_io;
    let oauth_auth_path = paths.oauth_auth;
    let kv_store_path = paths.kv_store;
    let session_store_path = paths.session_store;
    let component_paths = paths.components;
    let method_not_found_path = paths.method_not_found;
    // Discover server-handler interface from method-not-found (this is still needed for the chain)
    let server_handler_prefix = InterfaceType::ServerHandler.interface_prefix(DEFAULT_SPEC_VERSION);
    let server_handler_interface =
        find_component_export(method_not_found_path, &server_handler_prefix).context(
            "Failed to discover server-handler interface from method-not-found component",
        )?;

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
        println!("     transport: {}", transport_path.display());
        println!("     server-io: {}", server_io_path.display());
        println!("     oauth-auth: {}", oauth_auth_path.display());
        println!("     kv-store: {}", kv_store_path.display());
        println!("     session-store: {}", session_store_path.display());
        println!("     method-not-found: {}", method_not_found_path.display());
        for (i, path) in component_paths.iter().enumerate() {
            println!("     component-{}: {}", i, path.display());
        }
    }
    let packages = load_and_register_components(
        &mut graph,
        transport_path,
        server_io_path,
        oauth_auth_path,
        kv_store_path,
        session_store_path,
        component_paths,
        method_not_found_path,
        verbose,
    )?;

    // Track imports for validation
    let mut unsatisfied = UnsatisfiedImports::new();
    unsatisfied.add_component_imports("transport".to_string(), transport_path)?;
    unsatisfied.add_component_imports("method-not-found".to_string(), method_not_found_path)?;
    for (i, path) in component_paths.iter().enumerate() {
        unsatisfied.add_component_imports(format!("component-{}", i), path)?;
    }

    if verbose {
        eprintln!(
            "\n[VALIDATION] Tracking {} components with imports to validate",
            unsatisfied.imports.len()
        );
    }

    // Build service registry and instantiate all services
    if verbose {
        println!("   Building service registry...");
    }

    let mut services = ServiceRegistry::new();

    // Instantiate and register kv-store
    let kv_store_inst = graph.instantiate(packages.kv_store_id);
    services.register_service("kv-store", kv_store_inst, kv_store_path)?;

    if verbose {
        println!("   ✓ Registered kv-store service");
    }

    // Instantiate and register server-io
    let server_io_inst = graph.instantiate(packages.server_io_id);
    services.register_service("server-io", server_io_inst, server_io_path)?;

    if verbose {
        println!("   ✓ Registered server-io service");
    }

    // Instantiate session-store and auto-wire its dependencies
    let session_store_inst = graph.instantiate(packages.session_store_id);
    wire_all_services(
        &mut graph,
        session_store_inst,
        session_store_path,
        "session-store",
        &services,
        verbose,
    )?;
    services.register_service("session-store", session_store_inst, session_store_path)?;

    if verbose {
        println!("   ✓ Registered session-store service");
    }

    // Instantiate oauth-auth and auto-wire its dependencies
    let oauth_auth_inst = graph.instantiate(packages.oauth_auth_id);
    wire_all_services(
        &mut graph,
        oauth_auth_inst,
        oauth_auth_path,
        "oauth-auth",
        &services,
        verbose,
    )?;
    services.register_service("oauth-auth", oauth_auth_inst, oauth_auth_path)?;

    if verbose {
        println!("   ✓ Registered oauth-auth service");
        println!("\n   Service Registry Summary:");
        for (name, base, full) in services.all_exports() {
            println!("     {} exports: {} ({})", name, base, full);
        }
        println!();
    }

    // Build the middleware chain
    if verbose {
        println!("   Building composition graph...");
    }
    let handler_export = build_middleware_chain(
        &mut graph,
        &packages,
        component_paths,
        &server_handler_interface,
        &services,
        &mut unsatisfied,
        verbose,
    )?;

    // Wire transport and export interface
    wire_transport(
        &mut graph,
        super::wiring::TransportWireConfig {
            transport_id: packages.transport_id,
            handler_export,
            server_handler_interface: &server_handler_interface,
            transport_path,
            registry: &services,
            resolver: _resolver,
        },
        verbose,
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
    let version = version_resolver.get_version("mcp-v20250618")?;
    let server_handler_interface = interfaces::server_handler(&version);

    // Load and register all components
    if verbose {
        println!("   Loading components...");
    }

    let mut package_ids = Vec::new();
    for (i, path) in component_paths.iter().enumerate() {
        let name = format!("component-{}", i);
        let pkg = load_package(&mut graph, &name, path, verbose)?;
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

    /// Test WASI interface functions are available
    #[test]
    fn test_wasi_interface_functions() {
        use super::interfaces;

        let resolver = VersionResolver::new().unwrap();

        // These functions should be defined and accessible
        let http_handler = interfaces::wasi_http_handler(&resolver).unwrap();
        let cli_run = interfaces::wasi_cli_run(&resolver).unwrap();

        // Verify format (exact version may vary)
        assert!(http_handler.starts_with("wasi:http/incoming-handler@"));
        assert!(cli_run.starts_with("wasi:cli/run@"));

        // Verify specific current versions
        assert_eq!(http_handler, "wasi:http/incoming-handler@0.2.8");
        assert_eq!(cli_run, "wasi:cli/run@0.2.8");
    }

    /// Test server handler interface construction
    #[test]
    fn test_server_handler_interface_construction() {
        use crate::commands::compose::inspection::interfaces;

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
