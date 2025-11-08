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
use self::composition::{build_composition, build_handler_composition, wrap_capabilities};
use self::config::{resolve_output_path, validate_output_file, validate_transport};
use self::output::{
    print_handler_pipeline_diagram, print_handler_success_message, print_pipeline_diagram,
    print_success_message,
};
use self::resolution::{
    DownloadConfig, download_dependencies, resolve_kv_store_component,
    resolve_method_not_found_component, resolve_server_io_component,
    resolve_session_store_component, resolve_transport_component,
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

    /// Override transport component (path or package spec)
    pub override_transport: Option<String>,

    /// Override server-io component (path or package spec)
    pub override_server_io: Option<String>,

    /// Override kv-store component (path or package spec)
    pub override_kv_store: Option<String>,

    /// Override session-store component (path or package spec)
    pub override_session_store: Option<String>,

    /// Override method-not-found component (path or package spec)
    pub override_method_not_found: Option<String>,

    /// Override tools-middleware component (path or package spec)
    pub override_tools_middleware: Option<String>,

    /// Override resources-middleware component (path or package spec)
    pub override_resources_middleware: Option<String>,

    /// Override prompts-middleware component (path or package spec)
    pub override_prompts_middleware: Option<String>,

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
    let ComposeOptions {
        components,
        transport,
        output,
        version_resolver,
        override_transport,
        override_server_io,
        override_kv_store,
        override_session_store,
        override_method_not_found,
        override_tools_middleware,
        override_resources_middleware,
        override_prompts_middleware,
        deps_dir,
        skip_download,
        force,
        verbose,
        mode,
        runtime,
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
                override_server_io,
                override_kv_store,
                override_session_store,
                override_method_not_found,
                override_tools_middleware,
                override_resources_middleware,
                override_prompts_middleware,
                deps_dir,
                skip_download,
                force,
                verbose,
                runtime,
            )
            .await
        }
        CompositionMode::Handler => {
            compose_handler(
                components,
                output,
                version_resolver,
                override_tools_middleware,
                override_resources_middleware,
                override_prompts_middleware,
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
    override_server_io: Option<String>,
    override_kv_store: Option<String>,
    override_session_store: Option<String>,
    override_method_not_found: Option<String>,
    override_tools_middleware: Option<String>,
    override_resources_middleware: Option<String>,
    override_prompts_middleware: Option<String>,
    deps_dir: PathBuf,
    skip_download: bool,
    force: bool,
    verbose: bool,
    runtime: String,
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

    // Download framework dependencies once upfront (unless skip_download is set)
    // Skip downloading components that have overrides provided
    if !skip_download {
        if verbose {
            println!("\nDownloading framework dependencies...");
        }
        let download_config = DownloadConfig::from_overrides(
            &version_resolver,
            override_transport.as_deref(),
            override_server_io.as_deref(),
            override_kv_store.as_deref(),
            override_session_store.as_deref(),
            override_method_not_found.as_deref(),
            override_tools_middleware.as_deref(),
            override_resources_middleware.as_deref(),
            override_prompts_middleware.as_deref(),
        );
        download_dependencies(&download_config, &deps_dir, &client).await?;
    }

    // Resolve transport component
    let transport_path = resolve_transport_component(
        override_transport.as_deref(),
        &version_resolver,
        &deps_dir,
        &client,
        verbose,
    )
    .await?;

    // Resolve server-io component
    let server_io_path = resolve_server_io_component(
        override_server_io.as_deref(),
        &version_resolver,
        &deps_dir,
        &client,
        verbose,
    )
    .await?;

    // Resolve kv-store component
    let kv_store_path = resolve_kv_store_component(
        override_kv_store.as_deref(),
        &version_resolver,
        &deps_dir,
        &client,
        verbose,
        &runtime,
    )
    .await?;

    // Resolve session-store component (unified, no longer runtime-specific)
    let session_store_path = resolve_session_store_component(
        override_session_store.as_deref(),
        &version_resolver,
        &deps_dir,
        &client,
        verbose,
    )
    .await?;

    // Resolve method-not-found component
    let method_not_found_path = resolve_method_not_found_component(
        override_method_not_found.as_deref(),
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
        override_tools_middleware.as_deref(),
        override_resources_middleware.as_deref(),
        override_prompts_middleware.as_deref(),
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
        &transport_path,
        &server_io_path,
        &kv_store_path,
        &session_store_path,
        &wrapped_components,
        &method_not_found_path,
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
///
/// TODO: Refactor to reduce argument count (9/7). Consider grouping into a
/// HandlerCompositionOptions struct (components, overrides, paths, flags).
#[allow(clippy::too_many_arguments)]
async fn compose_handler(
    components: Vec<String>,
    output: PathBuf,
    version_resolver: VersionResolver,
    override_tools_middleware: Option<String>,
    override_resources_middleware: Option<String>,
    override_prompts_middleware: Option<String>,
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

    // Auto-detect and wrap capability components (tools, resources, etc.)
    if verbose {
        println!("\nDetecting component types...");
    }
    let wrapped_components = wrap_capabilities(
        component_paths,
        &deps_dir,
        &version_resolver,
        override_tools_middleware.as_deref(),
        override_resources_middleware.as_deref(),
        override_prompts_middleware.as_deref(),
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
