//! Component composition for MCP server pipelines
//!
//! This module provides functionality to compose MCP server components into
//! a complete WebAssembly component using the Component Model's composition
//! features via wac-graph.
//!
//! ## Architecture
//!
//! The MCP server architecture uses a universal middleware pattern where all
//! components communicate through the `wasmcp:handler/server-handler` interface.
//! This enables dynamic composition of arbitrary handlers into linear pipelines:
//!
//! ```text
//! transport → handler₁ → handler₂ → ... → handlerₙ → method-not-found
//! ```
//!
//! Each handler:
//! - Imports `server-handler` (to delegate unknown requests downstream)
//! - Exports `server-handler` (to handle requests from upstream)
//! - Optionally handles specific MCP methods (tools, resources, prompts, etc.)
//!
//! This uniform interface eliminates the need for handler type detection or
//! specialized wiring logic - composition is simply plugging components together
//! in sequence.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use wac_graph::{CompositionGraph, EncodeOptions};

use crate::pkg;

/// WIT interface constants for MCP protocol
mod interfaces {
    /// WASI HTTP incoming-handler interface (HTTP transport export)
    pub const WASI_HTTP_HANDLER: &str = "wasi:http/incoming-handler@0.2.3";

    /// WASI CLI run interface (stdio transport export)
    pub const WASI_CLI_RUN: &str = "wasi:cli/run@0.2.3";

    /// Generate the server-handler interface name with version
    pub fn server_handler(version: &str) -> String {
        format!("wasmcp:mcp/server-handler@{}", version)
    }

    /// Generate a versioned package name for wasmcp components
    pub fn package(name: &str, version: &str) -> String {
        format!("wasmcp:{}@{}", name, version)
    }
}

/// Configuration options for component composition
pub struct ComposeOptions {
    /// Ordered list of middleware component specs (paths or package names)
    pub components: Vec<String>,

    /// Transport type: "http" or "stdio"
    pub transport: String,

    /// Output path for the composed component
    pub output: PathBuf,

    /// wasmcp version for transport components
    pub version: String,

    /// Override transport component (path or package spec)
    pub override_transport: Option<String>,

    /// Override method-not-found component (path or package spec)
    pub override_method_not_found: Option<String>,

    /// Directory for downloaded dependencies
    pub deps_dir: PathBuf,

    /// Whether to skip downloading dependencies (use existing files)
    pub skip_download: bool,

    /// Whether to overwrite existing output file
    pub force: bool,
}

/// Compose MCP server components into a complete WASM component
///
/// This is the main entry point for the compose command. It:
/// 1. Resolves all component specs (downloading if needed)
/// 2. Downloads the appropriate transport component
/// 3. Chains components using wac-graph: transport → components → method-not-found
/// 4. Writes the composed component to the output path
///
/// # Errors
///
/// Returns an error if:
/// - Output file exists and --force is not set
/// - No components are specified
/// - Component resolution fails (file not found, download error)
/// - Component composition fails (invalid component, interface mismatch)
/// - Output file cannot be written
pub async fn compose(options: ComposeOptions) -> Result<()> {
    let ComposeOptions {
        components,
        transport,
        output,
        version,
        override_transport,
        override_method_not_found,
        deps_dir,
        skip_download,
        force,
    } = options;

    // Validate output file doesn't exist (unless force is set)
    if output.exists() && !force {
        anyhow::bail!(
            "Output file '{}' already exists. Use --force to overwrite.",
            output.display()
        );
    }

    // Require at least one component
    if components.is_empty() {
        anyhow::bail!(
            "No components specified. Provide one or more component paths or package specs.\n\
             Example: wasmcp compose my-handler.wasm namespace:other-handler@1.0.0"
        );
    }

    // Create package client for downloading components
    let cache_dir = deps_dir.join(".cache");
    let client = pkg::create_client(&cache_dir)
        .await
        .context("Failed to create package client")?;

    // Resolve all component specs to local paths
    println!("🔍 Resolving components...");
    let mut component_paths = Vec::new();

    for (i, spec) in components.iter().enumerate() {
        let path = resolve_component_spec(spec, &deps_dir, &client).await?;
        println!("   {}. {} → {}", i + 1, spec, path.display());
        component_paths.push(path);
    }

    // Resolve transport component (override or default)
    let transport_path = if let Some(override_spec) = override_transport {
        println!("\n🔧 Using override transport: {}", override_spec);
        resolve_component_spec(&override_spec, &deps_dir, &client).await?
    } else {
        // Download default transport if needed
        if !skip_download {
            println!("\n📦 Downloading framework dependencies...");
            download_dependencies(&transport, &version, &deps_dir, &client).await?;
        }
        let transport_name = format!("{}-transport", transport);
        get_dependency_path(&transport_name, &version, &deps_dir)?
    };

    // Resolve method-not-found component (override or default)
    let method_not_found_path = if let Some(override_spec) = override_method_not_found {
        println!("🔧 Using override method-not-found: {}", override_spec);
        resolve_component_spec(&override_spec, &deps_dir, &client).await?
    } else {
        // Download default method-not-found if needed
        if !skip_download {
            // Only download if we haven't already (transport download includes it)
            let method_not_found_pkg = interfaces::package("method-not-found", &version);
            let filename = method_not_found_pkg.replace([':', '/'], "_") + ".wasm";
            let path = deps_dir.join(&filename);
            if !path.exists() {
                download_dependencies(&transport, &version, &deps_dir, &client).await?;
            }
        }
        get_dependency_path("method-not-found", &version, &deps_dir)?
    };

    // Auto-detect and wrap tools-capability components
    println!("\n🔍 Detecting component types...");
    let wrapped_components = wrap_tools_capabilities(component_paths, &deps_dir, &version).await?;

    // Build the composition
    println!("\n🔧 Composing MCP server pipeline...");
    println!("   transport ({})", transport);
    for (i, path) in wrapped_components.iter().enumerate() {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("component");
        println!("   ↓");
        println!("   {}. {}", i + 1, name);
    }
    println!("   ↓");
    println!("   method-not-found");

    let bytes = build_composition(
        &transport_path,
        &wrapped_components,
        &method_not_found_path,
        &transport,
        &version,
    )
    .await?;

    // Write output file
    std::fs::write(&output, bytes)
        .context(format!("Failed to write output file: {}", output.display()))?;

    println!("\n✅ Composed: {}", output.display());
    println!("\nTo run the server:");
    match transport.as_str() {
        "http" => println!("  wasmtime serve -Scli {}", output.display()),
        "stdio" => println!("  wasmtime run {}", output.display()),
        _ => println!("  wasmtime {}", output.display()),
    }

    Ok(())
}

/// Resolve a component spec (path or package spec) to a local file path
///
/// - If spec is a local path (contains /, \, or ends with .wasm), validates existence
/// - Otherwise treats as package spec and downloads using wasm-pkg-client
///
/// # Examples
///
/// ```text
/// ./my-handler.wasm              → ./my-handler.wasm (if exists)
/// /abs/path/handler.wasm         → /abs/path/handler.wasm (if exists)
/// wasmcp:calculator@0.1.0        → deps/wasmcp_calculator@0.1.0.wasm
/// namespace:name                 → deps/namespace_name@latest.wasm
/// ```
async fn resolve_component_spec(
    spec: &str,
    deps_dir: &Path,
    client: &wasm_pkg_client::caching::CachingClient<wasm_pkg_client::caching::FileCache>,
) -> Result<PathBuf> {
    // Check if spec looks like a file path
    if spec.contains('/') || spec.contains('\\') || spec.ends_with(".wasm") {
        let path = PathBuf::from(spec);
        if !path.exists() {
            anyhow::bail!("Component not found: {}", spec);
        }
        return Ok(path);
    }

    // Otherwise treat as package spec and download
    println!("      Downloading {} from registry...", spec);
    pkg::resolve_spec(spec, client, deps_dir)
        .await
        .context(format!("Failed to download component: {}", spec))
}

/// Download required framework dependencies (transport, method-not-found, and tools-middleware)
async fn download_dependencies(
    transport: &str,
    version: &str,
    deps_dir: &Path,
    client: &wasm_pkg_client::caching::CachingClient<wasm_pkg_client::caching::FileCache>,
) -> Result<()> {
    let transport_pkg = interfaces::package(&format!("{}-transport", transport), version);
    let method_not_found_pkg = interfaces::package("method-not-found", version);
    let tools_middleware_pkg = interfaces::package("tools-middleware", version);

    let specs = vec![transport_pkg, method_not_found_pkg, tools_middleware_pkg];

    pkg::download_packages(client, &specs, deps_dir).await
}

/// Get the file path for a framework dependency
///
/// Framework dependencies are always stored as `wasmcp_{name}@{version}.wasm`
fn get_dependency_path(name: &str, version: &str, deps_dir: &Path) -> Result<PathBuf> {
    let filename = format!("wasmcp_{}@{}.wasm", name, version);
    let path = deps_dir.join(&filename);

    if !path.exists() {
        anyhow::bail!(
            "Dependency '{}' not found at {}. Run without --skip-download.",
            name,
            path.display()
        );
    }

    Ok(path)
}

/// Auto-detect and wrap tools-capability components with tools-middleware
///
/// This function inspects each component to determine if it exports tools-capability.
/// If so, it wraps the component with tools-middleware to convert it into a
/// server-handler component that can be composed into the pipeline.
async fn wrap_tools_capabilities(
    component_paths: Vec<PathBuf>,
    deps_dir: &Path,
    version: &str,
) -> Result<Vec<PathBuf>> {
    let mut wrapped_paths = Vec::new();
    let tools_cap_interface = format!("wasmcp:mcp/tools-capability@{}", version);

    for (i, path) in component_paths.iter().enumerate() {
        let component_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("component");

        // Check if this component exports tools-capability
        if component_exports_interface(path, &tools_cap_interface)? {
            println!("   {} is a tools-capability → wrapping with tools-middleware", component_name);

            // Get tools-middleware path
            let middleware_path = get_dependency_path("tools-middleware", version, deps_dir)?;

            // Wrap the capability with middleware
            let wrapped_bytes = wrap_with_tools_middleware(&middleware_path, path, version)?;

            // Write wrapped component to temp file
            let wrapped_path = deps_dir.join(format!(".wrapped-tools-{}.wasm", i));
            std::fs::write(&wrapped_path, wrapped_bytes)
                .context("Failed to write wrapped component")?;

            wrapped_paths.push(wrapped_path);
        } else {
            println!("   {} is a server-handler → using as-is", component_name);
            wrapped_paths.push(path.clone());
        }
    }

    Ok(wrapped_paths)
}

/// Check if a component exports a specific interface
///
/// This loads the component and inspects its exports to determine its type.
fn component_exports_interface(path: &Path, interface: &str) -> Result<bool> {
    use wasmparser::{Payload, Parser};

    let bytes = std::fs::read(path)
        .context(format!("Failed to read component: {}", path.display()))?;

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

/// Wrap a tools-capability component with tools-middleware
///
/// This composes: tools-middleware + tools-capability → wrapped component
/// The wrapped component exports server-handler and can be used in the pipeline.
fn wrap_with_tools_middleware(
    middleware_path: &Path,
    capability_path: &Path,
    version: &str,
) -> Result<Vec<u8>> {
    let mut graph = CompositionGraph::new();

    // Load both components
    let middleware_pkg = load_package(&mut graph, "tools-middleware", middleware_path)?;
    let capability_pkg = load_package(&mut graph, "tools-capability", capability_path)?;

    // Register packages
    let middleware_id = graph.register_package(middleware_pkg)?;
    let capability_id = graph.register_package(capability_pkg)?;

    // Get interface names
    let tools_cap_interface = format!("wasmcp:mcp/tools-capability@{}", version);
    let server_handler_interface = interfaces::server_handler(version);

    // Instantiate capability component
    let capability_inst = graph.instantiate(capability_id);

    // Get its tools-capability export
    let tools_cap_export = graph
        .alias_instance_export(capability_inst, &tools_cap_interface)
        .context("Failed to get tools-capability export")?;

    // Instantiate middleware
    let middleware_inst = graph.instantiate(middleware_id);

    // Wire middleware's tools-capability import to the capability's export
    graph
        .set_instantiation_argument(middleware_inst, &tools_cap_interface, tools_cap_export)
        .context("Failed to wire tools-capability")?;

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
async fn build_composition(
    transport_path: &Path,
    component_paths: &[PathBuf],
    method_not_found_path: &Path,
    transport_type: &str,
    version: &str,
) -> Result<Vec<u8>> {
    let mut graph = CompositionGraph::new();

    // Load all components as packages
    println!("   Loading components...");

    let transport_pkg = load_package(&mut graph, "transport", transport_path)?;
    let method_not_found_pkg = load_package(&mut graph, "method-not-found", method_not_found_path)?;

    let mut user_packages = Vec::new();
    for (i, path) in component_paths.iter().enumerate() {
        // Use index to ensure unique names even if components have same filename
        let name = format!("component-{}", i);
        let pkg = load_package(&mut graph, &name, path)?;
        user_packages.push(pkg);
    }

    // Register all packages with the graph
    println!("   Building composition graph...");

    let transport_pkg_id = graph.register_package(transport_pkg)?;
    let method_not_found_pkg_id = graph.register_package(method_not_found_pkg)?;

    let mut user_pkg_ids = Vec::new();
    for pkg in user_packages {
        user_pkg_ids.push(graph.register_package(pkg)?);
    }

    // Get the versioned server-handler interface name
    let server_handler_interface = interfaces::server_handler(version);

    // Start with method-not-found as the terminal handler
    let prev_inst = graph.instantiate(method_not_found_pkg_id);

    // Get the server-handler export from method-not-found
    let mut next_handler_export = graph
        .alias_instance_export(prev_inst, &server_handler_interface)
        .context("Failed to get server-handler export from method-not-found")?;

    // Chain user components in reverse order
    // This ensures when called, the first component processes first
    for (i, pkg_id) in user_pkg_ids.iter().enumerate().rev() {
        let inst = graph.instantiate(*pkg_id);

        // Wire this component's server-handler import to the previous component's export
        graph
            .set_instantiation_argument(inst, &server_handler_interface, next_handler_export)
            .context(format!("Failed to wire component-{} import", i))?;

        // This component's export becomes the next input
        next_handler_export = graph
            .alias_instance_export(inst, &server_handler_interface)
            .context(format!("Failed to get server-handler export from component-{}", i))?;
    }

    // Wire transport at the front of the chain
    let transport_inst = graph.instantiate(transport_pkg_id);
    graph.set_instantiation_argument(
        transport_inst,
        &server_handler_interface,
        next_handler_export,
    )?;

    // Export the appropriate WASI interface based on transport type
    match transport_type {
        "http" => {
            let http_handler =
                graph.alias_instance_export(transport_inst, interfaces::WASI_HTTP_HANDLER)?;
            graph.export(http_handler, interfaces::WASI_HTTP_HANDLER)?;
        }
        "stdio" => {
            let cli_run = graph.alias_instance_export(transport_inst, interfaces::WASI_CLI_RUN)?;
            graph.export(cli_run, interfaces::WASI_CLI_RUN)?;
        }
        _ => anyhow::bail!("Unsupported transport type: {}", transport_type),
    }

    // Encode the composition graph into a WebAssembly component
    println!("   Encoding component...");
    let bytes = graph
        .encode(EncodeOptions::default())
        .context("Failed to encode composition")?;

    Ok(bytes)
}

/// Load a WebAssembly component as a package in the composition graph
///
/// This reads the component file and registers it with wac-graph's type system.
fn load_package(
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
    .context(format!(
        "Failed to load component '{}' from {}",
        name,
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_names() {
        assert_eq!(
            interfaces::server_handler("0.4.0"),
            "wasmcp:mcp/server-handler@0.4.0"
        );
        assert_eq!(
            interfaces::WASI_HTTP_HANDLER,
            "wasi:http/incoming-handler@0.2.3"
        );
        assert_eq!(interfaces::WASI_CLI_RUN, "wasi:cli/run@0.2.3");
    }

    #[test]
    fn test_package_naming() {
        assert_eq!(
            interfaces::package("http-transport", "0.4.0"),
            "wasmcp:http-transport@0.4.0"
        );
        assert_eq!(
            interfaces::package("method-not-found", "0.4.0"),
            "wasmcp:method-not-found@0.4.0"
        );
    }
}
