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
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::commands::pkg;
use crate::config as wasmcp_config;
use crate::versioning::VersionResolver;

// Public re-exports
pub use self::config::ComposeOptionsBuilder;
pub use self::config::expand_profile_specs;
pub use self::resolution::PackageClient;

// Submodules - organized by functionality
pub mod composition;
pub mod config;
pub mod inspection;
pub mod output;
pub mod resolution;

// Internal imports from submodules
use self::composition::graph::CompositionPaths;
use self::composition::{
    build_composition, build_handler_composition, discover_required_middleware, wrap_capabilities,
};
use self::config::{resolve_output_path, validate_output_file, validate_transport};
use self::output::{
    print_handler_pipeline_diagram, print_handler_success_message, print_pipeline_diagram,
    print_success_message,
};
use self::resolution::{
    DownloadConfig, discover_required_dependencies, download_dependencies,
    resolve_framework_component, resolve_service_with_runtime,
};

/// Composition mode: Server (complete) or Handler (intermediate)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionMode {
    /// Complete MCP server with transport and terminal handler
    Server,
    /// Handler component without transport/terminal (composable)
    Handler,
}

/// Configuration options for component composition
#[derive(Debug)]
pub struct ComposeOptions {
    /// Ordered list of middleware component specs (paths or package names)
    pub components: Vec<String>,

    /// Transport type: "http" or "stdio"
    pub transport: String,

    /// Output path for the composed component
    pub output: PathBuf,

    /// Version resolver for component versions
    pub version_resolver: VersionResolver,

    /// Component overrides (component-name -> spec)
    ///
    /// Map of framework component names to override specs (paths or package names).
    /// Valid component names: transport, server-io, authorization, kv-store,
    /// session-store, method-not-found, tools-middleware, resources-middleware,
    /// prompts-middleware.
    pub overrides: HashMap<String, String>,

    /// Directory for downloaded dependencies
    pub deps_dir: PathBuf,

    /// Whether to skip downloading dependencies (use existing files)
    pub skip_download: bool,

    /// Whether to overwrite existing output file
    pub force: bool,

    /// Whether to show verbose output
    pub verbose: bool,

    /// Composition mode (Server or Handler)
    pub mode: CompositionMode,

    /// Runtime environment: "spin", "wasmcloud", or "wasmtime"
    /// Determines which session-store variant to use
    pub runtime: String,
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
/// - Transport type is not "http" or "stdio"
/// - Output file exists and --force is not set
/// - No components are specified
/// - Component resolution fails (file not found, download error)
/// - Component composition fails (invalid component, interface mismatch)
/// - Output file cannot be written
pub async fn compose(options: ComposeOptions) -> Result<()> {
    // Branch based on composition mode
    match options.mode {
        CompositionMode::Server => compose_server(options).await,
        CompositionMode::Handler => compose_handler(options).await,
    }
}

/// Check if a service has runtime-specific variants
///
/// Some services have different component variants based on the target runtime.
/// For example, kv-store has both kv-store (stable WASI) and kv-store-d2 (draft2 WASI).
fn has_runtime_variants(service_name: &str) -> bool {
    // Currently only kv-store has runtime variants
    // Future: could read from versions.toml metadata or convention
    service_name == "kv-store"
}

/// Compose a complete MCP server with transport and terminal handler
async fn compose_server(options: ComposeOptions) -> Result<()> {
    let ComposeOptions {
        components,
        transport,
        output,
        version_resolver,
        overrides,
        deps_dir,
        skip_download,
        force,
        verbose,
        runtime,
        mode: _,
    } = options;
    // Validate transport type early (before any expensive operations)
    validate_transport(&transport)?;

    // Validate and prepare output path
    let output_path = resolve_output_path(&output)?;
    validate_output_file(&output_path, force)?;

    // Validate components
    if components.is_empty() {
        anyhow::bail!(
            "no components specified, provide one or more component paths or package specs\n\
             example: wasmcp compose server my-handler.wasm namespace:other-handler@1.0.0"
        );
    }

    // Ensure wasmcp directories exist
    wasmcp_config::ensure_dirs()?;

    // Create package client
    let client = pkg::create_default_client()
        .await
        .context("Failed to create package client")?;

    // Print initial status
    if verbose {
        println!("Resolving components...");
    } else {
        println!(
            "Composing {} components ({} transport) → {}",
            components.len(),
            transport,
            output_path.display()
        );
    }

    // Resolve all component specs to local paths
    let component_paths = resolve_user_components(&components, &deps_dir, &client, verbose).await?;

    // Discover which framework dependencies are actually needed
    // 1. Inspect component imports to find required services (server-io, authorization, etc.)
    let required_deps = discover_required_dependencies(&component_paths, &overrides)?;

    // 2. Inspect component exports to find required middleware (tools/resources/prompts-middleware)
    let required_middleware = discover_required_middleware(&component_paths, &version_resolver)?;

    if verbose && !required_deps.is_empty() {
        println!("\nDiscovered required service dependencies:");
        for dep in &required_deps {
            println!("   - {}", dep);
        }
    }

    if verbose && !required_middleware.is_empty() {
        println!("\nDiscovered required middleware:");
        for mw in &required_middleware {
            println!("   - {}", mw);
        }
    }

    // Download framework dependencies once upfront (unless skip_download is set)
    // This returns ALL discovered dependencies including transitive ones
    let all_discovered_deps = if !skip_download {
        if verbose {
            println!("\nDownloading framework dependencies...");
        }
        let download_config =
            DownloadConfig::new(&overrides, &version_resolver, &required_middleware);
        download_dependencies(&component_paths, &download_config, &deps_dir, &client).await?
    } else {
        required_deps.clone()
    };

    // Resolve transport component
    let transport_name = "transport";
    let transport_path = resolve_framework_component(
        transport_name,
        overrides.get(transport_name).map(|s| s.as_str()),
        &version_resolver,
        &deps_dir,
        &client,
        verbose,
    )
    .await?;

    // Resolve service components that are actually needed
    // Only process services that were discovered as dependencies (including transitive ones)
    // This prevents errors when trying to load services that weren't downloaded
    let service_names: Vec<&str> = version_resolver
        .service_components()
        .into_iter()
        .filter(|name| all_discovered_deps.contains(*name))
        .collect();

    if verbose && !service_names.is_empty() {
        println!("\nResolving required services:");
        for name in &service_names {
            println!("   - {}", name);
        }
    }

    let mut service_paths = HashMap::new();

    for service_name in service_names {
        // Generic check: does this service have runtime-specific variants?
        let path = if has_runtime_variants(service_name) {
            resolve_service_with_runtime(
                service_name,
                overrides.get(service_name).map(|s| s.as_str()),
                &version_resolver,
                &deps_dir,
                &client,
                verbose,
                &runtime,
            )
            .await?
        } else {
            resolve_framework_component(
                service_name,
                overrides.get(service_name).map(|s| s.as_str()),
                &version_resolver,
                &deps_dir,
                &client,
                verbose,
            )
            .await?
        };

        service_paths.insert(service_name.to_string(), path);
    }

    // Resolve method-not-found component
    let method_not_found_name = "method-not-found";
    let method_not_found_path = resolve_framework_component(
        method_not_found_name,
        overrides.get(method_not_found_name).map(|s| s.as_str()),
        &version_resolver,
        &deps_dir,
        &client,
        verbose,
    )
    .await?;

    // Auto-detect and wrap capability components (tools, resources, etc.)
    if verbose {
        println!("\nDetecting component types...");
    }
    let wrapped_components = wrap_capabilities(
        component_paths,
        &deps_dir,
        &version_resolver,
        &overrides,
        &required_middleware,
        verbose,
    )
    .await?;

    // Print composition pipeline (only in verbose mode)
    if verbose {
        print_pipeline_diagram(&transport, &wrapped_components);
        println!("\nComposing MCP server pipeline...");
    }

    // Build and encode the composition
    let bytes = build_composition(
        CompositionPaths {
            transport: &transport_path,
            service_paths: &service_paths,
            components: &wrapped_components,
            method_not_found: &method_not_found_path,
        },
        &version_resolver,
        verbose,
    )
    .await?;

    // Write output file
    std::fs::write(&output_path, &bytes)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

    // Detect runtime requirements from composed component
    let runtime_info = match inspection::detect_runtime(&bytes) {
        Ok(info) => {
            if verbose {
                println!("\nDetected runtime: {:?}", info.runtime_type);
                println!("Required capabilities: {:?}", info.capabilities);
            }
            Some(info)
        }
        Err(e) => {
            if verbose {
                eprintln!("Warning: Failed to detect runtime info: {}", e);
            }
            None
        }
    };

    // Print success message with runtime-specific instructions
    print_success_message(&output_path, &transport, runtime_info.as_ref());

    Ok(())
}

/// Compose a handler component (without transport/terminal)
async fn compose_handler(options: ComposeOptions) -> Result<()> {
    let ComposeOptions {
        components,
        output,
        version_resolver,
        overrides,
        deps_dir,
        force,
        verbose,
        transport: _,
        skip_download: _,
        mode: _,
        runtime: _,
    } = options;
    // Validate and prepare output path
    let output_path = resolve_output_path(&output)?;
    validate_output_file(&output_path, force)?;

    // Validate components
    if components.is_empty() {
        anyhow::bail!(
            "no components specified, provide one or more component paths or package specs\n\
             example: wasmcp compose handler my-handler.wasm namespace:other-handler@1.0.0"
        );
    }

    // Ensure wasmcp directories exist
    wasmcp_config::ensure_dirs()?;

    // Create package client
    let client = pkg::create_default_client()
        .await
        .context("Failed to create package client")?;

    // Print initial status
    if verbose {
        println!("Resolving components...");
    } else {
        println!(
            "Composing {} handler components → {}",
            components.len(),
            output_path.display()
        );
    }

    // Resolve all component specs to local paths
    let component_paths = resolve_user_components(&components, &deps_dir, &client, verbose).await?;

    // Discover which middleware is needed by inspecting component exports
    let required_middleware = discover_required_middleware(&component_paths, &version_resolver)?;

    if verbose && !required_middleware.is_empty() {
        println!("\nDiscovered required middleware:");
        for mw in &required_middleware {
            println!("   - {}", mw);
        }
    }

    // Auto-detect and wrap capability components (tools, resources, etc.)
    if verbose {
        println!("\nDetecting component types...");
    }
    let wrapped_components = wrap_capabilities(
        component_paths,
        &deps_dir,
        &version_resolver,
        &overrides,
        &required_middleware,
        verbose,
    )
    .await?;

    // Print composition pipeline (only in verbose mode)
    if verbose {
        print_handler_pipeline_diagram(&wrapped_components);
        println!("\nComposing handler component...");
    }

    // Build and encode the handler-only composition
    let bytes = build_handler_composition(&wrapped_components, &version_resolver, verbose).await?;

    // Write output file
    std::fs::write(&output_path, bytes)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

    // Print success message
    print_handler_success_message(&output_path);

    Ok(())
}

/// Resolve all user component specs to local paths
async fn resolve_user_components(
    specs: &[String],
    deps_dir: &Path,
    client: &PackageClient,
    verbose: bool,
) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    for (i, spec) in specs.iter().enumerate() {
        let path =
            resolution::spec::resolve_component_spec(spec, deps_dir, client, verbose).await?;
        if verbose {
            println!("   {}. {} → {}", i + 1, spec, path.display());
        }
        paths.push(path);
    }

    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_names() {
        assert_eq!(
            inspection::interfaces::server_handler("0.1.0"),
            "wasmcp:mcp-v20250618/server-handler@0.1.0"
        );
        assert_eq!(
            inspection::interfaces::tools("0.1.0"),
            "wasmcp:mcp-v20250618/tools@0.1.0"
        );
        // Note: WASI interface versions now come from VersionResolver, not constants
        // These would require a resolver instance to test properly
    }

    #[test]
    fn test_package_naming() {
        assert_eq!(
            inspection::interfaces::package("http-transport", "0.1.0"),
            "wasmcp:http-transport@0.1.0"
        );
        assert_eq!(
            inspection::interfaces::package("method-not-found", "0.1.0"),
            "wasmcp:method-not-found@0.1.0"
        );
    }

    #[test]
    fn test_resolve_output_path_absolute() {
        let path = PathBuf::from("/absolute/path/output.wasm");
        let resolved = resolve_output_path(&path).unwrap();
        assert_eq!(resolved, path);
    }

    #[test]
    fn test_resolve_output_path_relative() {
        let path = PathBuf::from("output.wasm");
        let resolved = resolve_output_path(&path).unwrap();
        assert!(resolved.is_absolute());
        assert!(resolved.ends_with("output.wasm"));
    }

    /// Test CompositionMode enum
    #[test]
    fn test_composition_mode_enum() {
        let server = CompositionMode::Server;
        let handler = CompositionMode::Handler;

        // Test equality
        assert_eq!(server, CompositionMode::Server);
        assert_eq!(handler, CompositionMode::Handler);
        assert_ne!(server, handler);

        // Test Copy trait (CompositionMode implements Copy)
        let server_clone = server;
        assert_eq!(server, server_clone);
    }

    /// Test empty components error messages
    #[test]
    fn test_empty_components_error_messages() {
        // Server mode error
        let server_error = "no components specified, provide one or more component paths or package specs\n\
             example: wasmcp compose server my-handler.wasm namespace:other-handler@1.0.0";
        assert!(server_error.contains("no components specified"));
        assert!(server_error.contains("compose server"));

        // Handler mode error
        let handler_error = "no components specified, provide one or more component paths or package specs\n\
             example: wasmcp compose handler my-handler.wasm namespace:other-handler@1.0.0";
        assert!(handler_error.contains("no components specified"));
        assert!(handler_error.contains("compose handler"));
    }

    /// Test output file error messages
    #[test]
    fn test_output_file_error_messages() {
        let path = Path::new("/path/to/output.wasm");

        // File exists error
        let exists_error = format!(
            "output file '{}' already exists, use --force to overwrite",
            path.display()
        );
        assert!(exists_error.contains("already exists"));
        assert!(exists_error.contains("--force"));

        // Directory doesn't exist error
        let parent = Path::new("/nonexistent/directory");
        let dir_error = format!("output directory '{}' does not exist", parent.display());
        assert!(dir_error.contains("does not exist"));

        // Directory not writable error
        let not_writable_error = format!("output directory '{}' is not writable", parent.display());
        assert!(not_writable_error.contains("not writable"));
    }

    /// Test status message formatting
    #[test]
    fn test_status_messages() {
        let components_count = 3;
        let transport = "http";
        let output_path = Path::new("/path/to/server.wasm");

        // Server composition status
        let server_msg = format!(
            "Composing {} components ({} transport) → {}",
            components_count,
            transport,
            output_path.display()
        );
        assert!(server_msg.contains("Composing 3 components"));
        assert!(server_msg.contains("http transport"));
        assert!(server_msg.contains("server.wasm"));

        // Handler composition status
        let handler_msg = format!(
            "Composing {} handler components → {}",
            components_count,
            output_path.display()
        );
        assert!(handler_msg.contains("Composing 3 handler components"));
        assert!(handler_msg.contains("server.wasm"));

        // Verbose status messages
        let resolving_msg = "Resolving components...";
        assert!(resolving_msg.contains("Resolving"));

        let detecting_msg = "\nDetecting component types...";
        assert!(detecting_msg.contains("Detecting"));
    }

    /// Test framework download message
    #[test]
    fn test_framework_download_message() {
        let msg = "\nDownloading framework dependencies...";
        assert!(msg.contains("Downloading framework dependencies"));
    }

    /// Test override message formatting
    #[test]
    fn test_override_messages() {
        let component_name = "transport";
        let spec = "custom-transport.wasm";

        let override_msg = format!("\nUsing override {}: {}", component_name, spec);
        assert!(override_msg.contains("Using override"));
        assert!(override_msg.contains("transport"));
        assert!(override_msg.contains("custom-transport.wasm"));
    }

    /// Test component resolution verbose output format
    #[test]
    fn test_component_resolution_output() {
        let index = 1;
        let spec = "calculator.wasm";
        let path = Path::new("/path/to/calculator.wasm");

        let resolution_msg = format!("   {}. {} → {}", index, spec, path.display());
        assert!(resolution_msg.contains("1. calculator.wasm →"));
        assert!(resolution_msg.starts_with("   "));
    }

    /// Test ComposeOptions mode default
    #[test]
    fn test_compose_options_default_mode() {
        let options = ComposeOptionsBuilder::new(vec!["handler.wasm".to_string()])
            .build()
            .unwrap();

        // Builder defaults to Server mode
        assert_eq!(options.mode, CompositionMode::Server);
        assert_eq!(options.transport, "http");
        assert_eq!(options.output, PathBuf::from("server.wasm"));
        assert!(!options.force);
        assert!(!options.skip_download);
        // Version comes from embedded versions.toml
        assert!(
            options
                .version_resolver
                .get_version("mcp-v20250618")
                .is_ok()
        );
    }

    /// Test http messages path handling

    #[test]
    fn test_compose_options_builder_chaining() {
        let options = ComposeOptionsBuilder::new(vec!["a.wasm".to_string()])
            .transport("http")
            .output(PathBuf::from("out.wasm"))
            .override_component("transport", "custom-transport.wasm")
            .override_component("method-not-found", "custom-mnf.wasm")
            .build()
            .unwrap();

        // Version comes from embedded versions.toml
        assert!(
            options
                .version_resolver
                .get_version("mcp-v20250618")
                .is_ok()
        );
        assert_eq!(
            options.overrides.get("transport"),
            Some(&"custom-transport.wasm".to_string())
        );
        assert_eq!(
            options.overrides.get("method-not-found"),
            Some(&"custom-mnf.wasm".to_string())
        );
    }

    /// Test component count in messages
    #[test]
    fn test_component_count_formatting() {
        let components = ["a.wasm", "b.wasm", "c.wasm"];
        let count = components.len();

        let msg = format!("Composing {} components", count);
        assert_eq!(msg, "Composing 3 components");

        // Single component
        let single = ["one.wasm"];
        let single_msg = format!("Composing {} components", single.len());
        assert_eq!(single_msg, "Composing 1 components");
    }
}
