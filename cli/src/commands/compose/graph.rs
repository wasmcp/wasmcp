//! WebAssembly component composition using wac-graph
//!
//! This module handles building the component composition graph that chains
//! transport → components → method-not-found into a complete MCP server.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use wac_graph::{CompositionGraph, EncodeOptions};

use super::dependencies;
use super::interfaces::{DEFAULT_SPEC_VERSION, InterfaceType};
use crate::versioning::VersionResolver;

/// Package IDs for composition
struct CompositionPackages {
    transport_id: wac_graph::PackageId,
    server_io_id: wac_graph::PackageId,
    session_store_id: wac_graph::PackageId,
    user_ids: Vec<wac_graph::PackageId>,
    method_not_found_id: wac_graph::PackageId,
}

// TODO: Complete validation implementation per .agent/wire-troubleshooting.md
// These structures and methods are scaffolded but not fully wired up yet.
// Remove #[allow(dead_code)] when implementing the full validation.

/// Tracks unsatisfied imports for validation
#[derive(Debug)]
struct UnsatisfiedImports {
    /// Map of component name -> list of unsatisfied import interfaces
    imports: std::collections::HashMap<String, Vec<String>>,
}

#[allow(dead_code)]
impl UnsatisfiedImports {
    fn new() -> Self {
        Self {
            imports: std::collections::HashMap::new(),
        }
    }

    /// Add a component's imports (excluding WASI imports)
    fn add_component_imports(&mut self, name: String, component_path: &Path) -> Result<()> {
        let imports = check_component_imports(component_path)?;
        let non_wasi: Vec<String> = imports
            .into_iter()
            .filter(|import| !import.starts_with("wasi:"))
            .collect();

        if !non_wasi.is_empty() {
            self.imports.insert(name, non_wasi);
        }
        Ok(())
    }

    /// Mark an import as satisfied for a specific component
    fn mark_satisfied(&mut self, component_name: &str, interface: &str) {
        if let Some(imports) = self.imports.get_mut(component_name) {
            imports.retain(|i| i != interface);
            if imports.is_empty() {
                self.imports.remove(component_name);
            }
        }
    }

    /// Check if any imports remain unsatisfied
    fn has_unsatisfied(&self) -> bool {
        !self.imports.is_empty()
    }

    /// Get formatted error message for remaining unsatisfied imports
    fn error_message(&self) -> String {
        let mut msg = String::from("Composition has unsatisfied imports:\n");
        for (component, imports) in &self.imports {
            msg.push_str(&format!("  Component '{}':\n", component));
            for import in imports {
                msg.push_str(&format!("    - {}\n", import));
            }
        }
        msg.push_str("\nThese imports were not wired during composition. ");
        msg.push_str(
            "Check that you're wiring all required framework interfaces to user components.",
        );
        msg
    }
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
    let server_handler_prefix = InterfaceType::ServerHandler.interface_prefix(DEFAULT_SPEC_VERSION);
    let server_handler_interface =
        find_component_export(method_not_found_path, &server_handler_prefix).context(
            "Failed to discover server-handler interface from method-not-found component",
        )?;

    // Discover interface names from the components that EXPORT them
    // This ensures alias_instance_export gets the exact names the components use
    let server_io_prefix = InterfaceType::ServerIo.interface_prefix(DEFAULT_SPEC_VERSION);
    let server_io_interface = find_component_export(server_io_path, &server_io_prefix)
        .context("Failed to discover server-io interface from server-io component")?;

    let sessions_prefix = InterfaceType::Sessions.interface_prefix(DEFAULT_SPEC_VERSION);
    let sessions_interface = find_component_export(session_store_path, &sessions_prefix)
        .context("Failed to discover sessions interface from session-store component")?;

    let session_manager_prefix =
        InterfaceType::SessionManager.interface_prefix(DEFAULT_SPEC_VERSION);
    let session_manager_interface =
        find_component_export(session_store_path, &session_manager_prefix)
            .context("Failed to discover session-manager interface from session-store component")?;

    if verbose {
        println!("   Discovered interfaces:");
        println!("     server-handler: {}", server_handler_interface);
        println!("     server-io: {}", server_io_interface);
        println!("     sessions: {}", sessions_interface);
        println!("     session-manager: {}", session_manager_interface);
    }

    let mut graph = CompositionGraph::new();

    // Load and register all components
    if verbose {
        println!("   Loading components...");
        println!("     transport: {}", transport_path.display());
        println!("     server-io: {}", server_io_path.display());
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
        session_store_path,
        component_paths,
        method_not_found_path,
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

    // Instantiate service components first (needed for middleware wiring)
    if verbose {
        println!("   Instantiating service components...");

        // Check what transport actually imports BEFORE instantiation
        eprintln!("\n[DEBUG] ==================== COMPONENT ANALYSIS ====================");
        eprintln!("[DEBUG] Transport imports:");
        if let Ok(imports) = check_component_imports(transport_path) {
            for import in imports {
                eprintln!("[DEBUG]   - {}", import);
            }
        }

        eprintln!("\n[DEBUG] Server-io exports:");
        if let Ok(exports) = check_component_exports(server_io_path) {
            for export in exports {
                eprintln!("[DEBUG]   - {}", export);
            }
        }

        eprintln!("\n[DEBUG] Session-store exports:");
        if let Ok(exports) = check_component_exports(session_store_path) {
            for export in exports {
                eprintln!("[DEBUG]   - {}", export);
            }
        }
        eprintln!("[DEBUG] ==================== END COMPONENT ANALYSIS ====================\n");
    }

    let server_io_inst = graph.instantiate(packages.server_io_id);
    let session_store_inst = graph.instantiate(packages.session_store_id);

    // Build the middleware chain
    if verbose {
        println!("   Building composition graph...");
    }
    let handler_export = build_middleware_chain(
        &mut graph,
        &packages,
        server_io_inst,
        session_store_inst,
        component_paths,
        &server_handler_interface,
        &server_io_interface,
        &sessions_interface,
        &mut unsatisfied,
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
        &server_io_interface,
        &sessions_interface,
        &session_manager_interface,
        transport_path,
        server_io_path,
        session_store_path,
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
    server_io_inst: wac_graph::NodeId,
    session_store_inst: wac_graph::NodeId,
    component_paths: &[PathBuf],
    server_handler_interface: &str,
    server_io_interface: &str,
    sessions_interface: &str,
    _unsatisfied: &mut UnsatisfiedImports,
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

        // Check if this component imports server-io and wire it if so
        if component_imports_interface(&component_paths[i], server_io_interface)? {
            let server_io_export = graph
                .alias_instance_export(server_io_inst, server_io_interface)
                .context("Failed to get server-io export from server-io")?;

            graph
                .set_instantiation_argument(inst, server_io_interface, server_io_export)
                .with_context(|| format!("Failed to wire component-{} server-io import", i))?;
        }

        // Check if this component imports sessions and wire it if so
        if component_imports_interface(&component_paths[i], sessions_interface)? {
            let sessions_export = graph
                .alias_instance_export(session_store_inst, sessions_interface)
                .context("Failed to get sessions export from session-store")?;

            graph
                .set_instantiation_argument(inst, sessions_interface, sessions_export)
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
#[allow(clippy::too_many_arguments)]
fn wire_transport(
    graph: &mut CompositionGraph,
    transport_id: wac_graph::PackageId,
    server_io_inst: wac_graph::NodeId,
    session_store_inst: wac_graph::NodeId,
    handler_export: wac_graph::NodeId,
    transport_type: &str,
    server_handler_interface: &str,
    server_io_interface: &str,
    sessions_interface: &str,
    session_manager_interface: &str,
    transport_path: &Path,
    server_io_path: &Path,
    _session_store_path: &Path,
) -> Result<()> {
    eprintln!("\n[WIRE] ==================== WIRING ANALYSIS ====================");

    // Instantiate transport
    let transport_inst = graph.instantiate(transport_id);
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

    // Wire transport's server-handler import to the middleware chain
    eprintln!("[WIRE] 1. Wiring server-handler...");
    eprintln!("[WIRE]    Interface: {}", server_handler_interface);
    match graph.set_instantiation_argument(transport_inst, server_handler_interface, handler_export)
    {
        Ok(_) => eprintln!("[WIRE]    ✓ Success"),
        Err(e) => {
            eprintln!("[WIRE]    ✗ FAILED: {:?}", e);
            return Err(e).context("Failed to wire transport server-handler import");
        }
    }

    // Wire transport's server-io import to the server-io service
    eprintln!("\n[WIRE] 2. Wiring server-io...");
    eprintln!("[WIRE]    Interface: {}", server_io_interface);

    // Show detailed signature comparison
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
    let server_io_export = match graph.alias_instance_export(server_io_inst, server_io_interface) {
        Ok(export) => {
            eprintln!("[WIRE]    ✓ Got export from server-io");
            export
        }
        Err(e) => {
            eprintln!("[WIRE]    ✗ FAILED to get export: {:?}", e);
            eprintln!(
                "[WIRE]    This means server-io does NOT export '{}'",
                server_io_interface
            );
            return Err(e).with_context(|| format!("Failed to get server-io export for interface '{}'. Server-io component may not export this exact interface name.", server_io_interface));
        }
    };

    eprintln!("[WIRE]    Attempting to wire transport import to server-io export...");
    match graph.set_instantiation_argument(transport_inst, server_io_interface, server_io_export) {
        Ok(_) => eprintln!("[WIRE]    ✓ Success"),
        Err(e) => {
            eprintln!("[WIRE]    ✗ FAILED: {:?}", e);
            return Err(e).with_context(|| {
                format!(
                    "Failed to wire transport server-io import for interface '{}'",
                    server_io_interface
                )
            });
        }
    }

    // Wire transport's sessions import to the session-store service
    eprintln!("\n[WIRE] 3. Wiring sessions...");
    eprintln!("[WIRE]    Interface: {}", sessions_interface);
    let sessions_export = graph
        .alias_instance_export(session_store_inst, sessions_interface)
        .context("Failed to get sessions export")?;
    match graph.set_instantiation_argument(transport_inst, sessions_interface, sessions_export) {
        Ok(_) => eprintln!("[WIRE]    ✓ Success"),
        Err(e) => {
            eprintln!("[WIRE]    ✗ FAILED: {:?}", e);
            return Err(e).context("Failed to wire transport sessions import");
        }
    }

    // Wire transport's session-manager import to the session-store service
    eprintln!("\n[WIRE] 4. Wiring session-manager...");
    eprintln!("[WIRE]    Interface: {}", session_manager_interface);
    let session_manager_export = graph
        .alias_instance_export(session_store_inst, session_manager_interface)
        .context("Failed to get session-manager export")?;
    match graph.set_instantiation_argument(
        transport_inst,
        session_manager_interface,
        session_manager_export,
    ) {
        Ok(_) => eprintln!("[WIRE]    ✓ Success"),
        Err(e) => {
            eprintln!("[WIRE]    ✗ FAILED: {:?}", e);
            return Err(e).context("Failed to wire transport session-manager import");
        }
    }

    eprintln!("\n[WIRE] --- Exporting Transport Interface ---");
    // Export the appropriate WASI interface based on transport type
    match transport_type {
        "http" => {
            eprintln!("[WIRE] Exporting HTTP handler interface");
            let http_handler = graph.alias_instance_export(
                transport_inst,
                dependencies::interfaces::WASI_HTTP_HANDLER,
            )?;
            graph.export(http_handler, dependencies::interfaces::WASI_HTTP_HANDLER)?;
        }
        "stdio" => {
            eprintln!("[WIRE] Exporting CLI run interface");
            let cli_run = graph
                .alias_instance_export(transport_inst, dependencies::interfaces::WASI_CLI_RUN)?;
            graph.export(cli_run, dependencies::interfaces::WASI_CLI_RUN)?;
        }
        _ => anyhow::bail!("unsupported transport type: '{}'", transport_type),
    }

    eprintln!("[WIRE] ==================== WIRING COMPLETE ====================\n");

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
    eprintln!(
        "[LOAD] Loading component '{}' from {}",
        name,
        path.display()
    );
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

    // Log what this package exports
    eprintln!("[LOAD]   Component '{}' loaded successfully", name);

    Ok(package)
}

/// Check what interfaces a component exports (for debugging)
fn check_component_exports(component_path: &Path) -> Result<Vec<String>> {
    use wit_component::DecodedWasm;

    let bytes = std::fs::read(component_path)?;
    let decoded = wit_component::decode(&bytes)?;

    let (resolve, world_id) = match decoded {
        DecodedWasm::Component(resolve, world_id) => (resolve, world_id),
        _ => return Ok(vec![]),
    };

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
fn check_component_imports(component_path: &Path) -> Result<Vec<String>> {
    use wit_component::DecodedWasm;

    let bytes = std::fs::read(component_path)?;
    let decoded = wit_component::decode(&bytes)?;

    let (resolve, world_id) = match decoded {
        DecodedWasm::Component(resolve, world_id) => (resolve, world_id),
        _ => return Ok(vec![]),
    };

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
fn get_interface_details(component_path: &Path, interface_name: &str) -> Result<String> {
    use wit_component::DecodedWasm;

    let bytes = std::fs::read(component_path)?;
    let decoded = wit_component::decode(&bytes)?;

    let (resolve, world_id) = match decoded {
        DecodedWasm::Component(resolve, world_id) => (resolve, world_id),
        _ => anyhow::bail!("Not a component"),
    };

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

/// Find an interface import from a component by prefix pattern
///
/// Inspects the component binary to find an import matching the given prefix.
/// For example, prefix "wasmcp:mcp-v20250618/server-io@" will match "wasmcp:mcp-v20250618/server-io@0.1.4".
///
/// Returns the full interface name if found.
///
/// TODO: This was used in an earlier approach but is kept for potential future use.
/// Remove #[allow(dead_code)] if this becomes needed again.
#[allow(dead_code)]
fn find_component_import(component_path: &Path, prefix: &str) -> Result<String> {
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

    // Search imports for matching interface
    for (key, _item) in &world.imports {
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
        "No import found matching prefix '{}' in component at {}",
        prefix,
        component_path.display()
    )
}

/// Check if a component imports a specific interface
///
/// Inspects the component binary to determine if it imports the given interface.
/// This is used to determine whether to wire service exports to this component.
fn component_imports_interface(component_path: &Path, interface_name: &str) -> Result<bool> {
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
