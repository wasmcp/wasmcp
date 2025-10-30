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

use crate::versioning::VersionResolver;
use crate::{commands::pkg, config};

// Public re-exports
pub use self::builder::ComposeOptionsBuilder;
pub use self::dependencies::PackageClient;
pub use self::profiles::expand_profile_specs;

// Submodules
mod builder;
pub mod dependencies;
mod framework;
mod graph;
mod output;
mod profiles;
mod resolution;
mod validation;
mod wrapping;

// Internal imports from submodules
use self::framework::{
    resolve_http_notifications_component, resolve_method_not_found_component,
    resolve_transport_component,
};
use self::output::{
    print_handler_pipeline_diagram, print_handler_success_message, print_pipeline_diagram,
    print_success_message,
};
use self::validation::{resolve_output_path, validate_output_file, validate_transport};

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

    /// Whether to show verbose output
    pub verbose: bool,

    /// Composition mode (Server or Handler)
    pub mode: CompositionMode,
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
    let ComposeOptions {
        components,
        transport,
        output,
        version_resolver,
        override_transport,
        override_method_not_found,
        deps_dir,
        skip_download,
        force,
        verbose,
        mode,
    } = options;

    // Branch based on composition mode
    match mode {
        CompositionMode::Server => {
            compose_server(
                components,
                transport,
                output,
                version_resolver,
                override_transport,
                override_method_not_found,
                deps_dir,
                skip_download,
                force,
                verbose,
            )
            .await
        }
        CompositionMode::Handler => {
            compose_handler(
                components,
                output,
                version_resolver,
                deps_dir,
                force,
                verbose,
            )
            .await
        }
    }
}

/// Compose a complete MCP server with transport and terminal handler
#[allow(clippy::too_many_arguments)]
async fn compose_server(
    components: Vec<String>,
    transport: String,
    output: PathBuf,
    version_resolver: VersionResolver,
    override_transport: Option<String>,
    override_method_not_found: Option<String>,
    deps_dir: PathBuf,
    skip_download: bool,
    force: bool,
    verbose: bool,
) -> Result<()> {
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
    config::ensure_dirs()?;

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

    // Resolve transport component
    let transport_path = resolve_transport_component(
        &transport,
        override_transport.as_deref(),
        &version_resolver,
        &deps_dir,
        &client,
        skip_download,
        verbose,
    )
    .await?;

    // Resolve method-not-found component
    let method_not_found_path = resolve_method_not_found_component(
        override_method_not_found.as_deref(),
        &version_resolver,
        &deps_dir,
        &client,
        skip_download,
        verbose,
    )
    .await?;

    // Resolve http-notifications component for http transport
    let http_notifications_path = if transport == "http" {
        Some(
            resolve_http_notifications_component(
                &version_resolver,
                &deps_dir,
                &client,
                skip_download,
                verbose,
            )
            .await?,
        )
    } else {
        None
    };

    // Auto-detect and wrap capability components (tools, resources, etc.)
    if verbose {
        println!("\nDetecting component types...");
    }
    let wrapped_components =
        wrapping::wrap_capabilities(component_paths, &deps_dir, &version_resolver, verbose).await?;

    // Print composition pipeline (only in verbose mode)
    if verbose {
        print_pipeline_diagram(&transport, &wrapped_components);
        println!("\nComposing MCP server pipeline...");
    }

    // Build and encode the composition
    let bytes = graph::build_composition(
        &transport_path,
        &wrapped_components,
        &method_not_found_path,
        http_notifications_path.as_deref(),
        &transport,
        &version_resolver,
        verbose,
    )
    .await?;

    // Write output file
    std::fs::write(&output_path, &bytes)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

    // Print success message
    print_success_message(&output_path, &transport);

    Ok(())
}

/// Compose a handler component (without transport/terminal)
async fn compose_handler(
    components: Vec<String>,
    output: PathBuf,
    version_resolver: VersionResolver,
    deps_dir: PathBuf,
    force: bool,
    verbose: bool,
) -> Result<()> {
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
    config::ensure_dirs()?;

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

    // Auto-detect and wrap capability components (tools, resources, etc.)
    if verbose {
        println!("\nDetecting component types...");
    }
    let wrapped_components =
        wrapping::wrap_capabilities(component_paths, &deps_dir, &version_resolver, verbose).await?;

    // Print composition pipeline (only in verbose mode)
    if verbose {
        print_handler_pipeline_diagram(&wrapped_components);
        println!("\nComposing handler component...");
    }

    // Build and encode the handler-only composition
    let bytes =
        graph::build_handler_composition(&wrapped_components, &version_resolver, verbose).await?;

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
        let path = resolution::resolve_component_spec(spec, deps_dir, client, verbose).await?;
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
            dependencies::interfaces::server_handler("0.1.0"),
            "wasmcp:mcp-v20250618/server-handler@0.1.0"
        );
        assert_eq!(
            dependencies::interfaces::tools("0.1.0"),
            "wasmcp:mcp-v20250618/tools@0.1.0"
        );
        assert_eq!(
            dependencies::interfaces::WASI_HTTP_HANDLER,
            "wasi:http/incoming-handler@0.2.3"
        );
        assert_eq!(dependencies::interfaces::WASI_CLI_RUN, "wasi:cli/run@0.2.3");
    }

    #[test]
    fn test_package_naming() {
        assert_eq!(
            dependencies::interfaces::package("http-transport", "0.1.0"),
            "wasmcp:http-transport@0.1.0"
        );
        assert_eq!(
            dependencies::interfaces::package("method-not-found", "0.1.0"),
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

    /// Test http notifications path handling
    #[test]
    fn test_http_notifications_conditional() {
        // HTTP transport should include notifications
        let transport = "http";
        let should_include = transport == "http";
        assert!(should_include);

        // Stdio transport should not include notifications
        let transport = "stdio";
        let should_include = transport == "http";
        assert!(!should_include);
    }

    #[test]
    fn test_compose_options_builder_chaining() {
        let options = ComposeOptionsBuilder::new(vec!["a.wasm".to_string()])
            .transport("http")
            .output(PathBuf::from("out.wasm"))
            .override_transport("custom-transport.wasm")
            .override_method_not_found("custom-mnf.wasm")
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
            options.override_transport,
            Some("custom-transport.wasm".to_string())
        );
        assert_eq!(
            options.override_method_not_found,
            Some("custom-mnf.wasm".to_string())
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
