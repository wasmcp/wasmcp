//! Automatic detection and wrapping of capability components
//!
//! This module handles detecting whether components export capability interfaces
//! (tools, resources, etc.) and automatically wrapping them with the appropriate
//! middleware to convert them into server-handler components.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use wac_graph::{CompositionGraph, EncodeOptions};

use super::dependencies;

/// Prefix for temporary wrapped component files
const WRAPPED_COMPONENT_PREFIX: &str = ".wrapped-";

/// Auto-detect and wrap capability components with appropriate middleware
///
/// This function inspects each component to determine if it exports capability
/// interfaces (tools, resources, etc.). If so, it wraps the component with the
/// appropriate middleware to convert it into a server-handler component.
pub async fn wrap_capabilities(
    component_paths: Vec<PathBuf>,
    deps_dir: &Path,
    version: &str,
    verbose: bool,
) -> Result<Vec<PathBuf>> {
    let mut wrapped_paths = Vec::new();
    let tools_interface = dependencies::interfaces::tools(version);
    let resources_interface = dependencies::interfaces::resources(version);

    for (i, path) in component_paths.into_iter().enumerate() {
        let component_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("component");

        // Check for tools capability
        if component_exports_interface(&path, &tools_interface)? {
            if verbose {
                println!(
                    "   {} is a tools-capability → wrapping with tools-middleware",
                    component_name
                );
            }

            let middleware_path =
                dependencies::get_dependency_path("tools-middleware", version, deps_dir)?;
            let wrapped_bytes = wrap_with_middleware(
                &middleware_path,
                &path,
                &tools_interface,
                "tools-middleware",
                "tools-capability",
                version,
            )?;

            let wrapped_path =
                deps_dir.join(format!("{}tools-{}.wasm", WRAPPED_COMPONENT_PREFIX, i));
            std::fs::write(&wrapped_path, wrapped_bytes)
                .context("Failed to write wrapped component")?;

            wrapped_paths.push(wrapped_path);
        }
        // Check for resources capability
        else if component_exports_interface(&path, &resources_interface)? {
            if verbose {
                println!(
                    "   {} is a resources-capability → wrapping with resources-middleware",
                    component_name
                );
            }

            let middleware_path =
                dependencies::get_dependency_path("resources-middleware", version, deps_dir)?;
            let wrapped_bytes = wrap_with_middleware(
                &middleware_path,
                &path,
                &resources_interface,
                "resources-middleware",
                "resources-capability",
                version,
            )?;

            let wrapped_path =
                deps_dir.join(format!("{}resources-{}.wasm", WRAPPED_COMPONENT_PREFIX, i));
            std::fs::write(&wrapped_path, wrapped_bytes)
                .context("Failed to write wrapped component")?;

            wrapped_paths.push(wrapped_path);
        }
        // Not a capability component - use as-is
        else {
            if verbose {
                println!("   {} is a server-handler → using as-is", component_name);
            }
            wrapped_paths.push(path);
        }
    }

    Ok(wrapped_paths)
}

/// Check if a component exports a specific interface
///
/// This loads the component and inspects its exports to determine its type.
fn component_exports_interface(path: &Path, interface: &str) -> Result<bool> {
    use wasmparser::{Parser, Payload};

    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read component: {}", path.display()))?;

    // Parse the component to find exports
    for payload in Parser::new(0).parse_all(&bytes) {
        let payload = payload.context("Failed to parse component")?;

        if let Payload::ComponentExportSection(exports) = payload {
            for export in exports {
                let export = export.context("Failed to parse export")?;
                if export.name.0 == interface {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

/// Wrap a capability component with its middleware
///
/// This composes: middleware + capability → wrapped component
/// The wrapped component exports server-handler and can be used in the pipeline.
fn wrap_with_middleware(
    middleware_path: &Path,
    capability_path: &Path,
    capability_interface: &str,
    middleware_name: &str,
    capability_name: &str,
    version: &str,
) -> Result<Vec<u8>> {
    let mut graph = CompositionGraph::new();

    // Load both components
    let middleware_pkg = super::graph::load_package(&mut graph, middleware_name, middleware_path)?;
    let capability_pkg = super::graph::load_package(&mut graph, capability_name, capability_path)?;

    // Register packages
    let middleware_id = graph.register_package(middleware_pkg)?;
    let capability_id = graph.register_package(capability_pkg)?;

    // Get interface names
    let server_handler_interface = dependencies::interfaces::server_handler(version);

    // Instantiate capability component
    let capability_inst = graph.instantiate(capability_id);

    // Get its capability export (tools, resources, etc.)
    let capability_export = graph
        .alias_instance_export(capability_inst, capability_interface)
        .with_context(|| format!("Failed to get {} export", capability_name))?;

    // Instantiate middleware
    let middleware_inst = graph.instantiate(middleware_id);

    // Wire middleware's capability import to the capability's export
    graph
        .set_instantiation_argument(middleware_inst, capability_interface, capability_export)
        .with_context(|| format!("Failed to wire {} interface", capability_name))?;

    // Export the middleware's server-handler export
    let server_handler_export = graph
        .alias_instance_export(middleware_inst, &server_handler_interface)
        .context("Failed to get server-handler export from middleware")?;

    graph
        .export(server_handler_export, &server_handler_interface)
        .context("Failed to export server-handler")?;

    // Encode the wrapped component
    let bytes = graph
        .encode(EncodeOptions::default())
        .context("Failed to encode wrapped component")?;

    Ok(bytes)
}
