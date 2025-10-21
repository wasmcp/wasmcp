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

use crate::{commands::pkg, config};

// Public re-exports
pub use self::dependencies::PackageClient;
pub use self::profiles::expand_profile_specs;

// Submodules
pub mod dependencies;
mod graph;
mod profiles;
mod resolution;
mod wrapping;

/// Configuration options for component composition
#[derive(Debug, Clone)]
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

    /// Whether to show verbose output
    pub verbose: bool,
}

/// Builder for ComposeOptions with sensible defaults
///
/// This builder is part of the public API for programmatic use.
/// It's not currently used by the CLI but is available for external consumers.
///
/// # Examples
///
/// ```rust
/// use wasmcp::compose::ComposeOptionsBuilder;
/// use std::path::PathBuf;
///
/// # fn example() -> anyhow::Result<()> {
/// let options = ComposeOptionsBuilder::new(vec!["./handler.wasm".to_string()])
///     .transport("stdio")
///     .output(PathBuf::from("my-server.wasm"))
///     .force(true)
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
#[allow(dead_code)] // Public API - used by external consumers, not internally yet
pub struct ComposeOptionsBuilder {
    components: Vec<String>,
    transport: String,
    output: PathBuf,
    version: String,
    override_transport: Option<String>,
    override_method_not_found: Option<String>,
    deps_dir: Option<PathBuf>,
    skip_download: bool,
    force: bool,
    verbose: bool,
}

#[allow(dead_code)] // Public API - used by external consumers, not internally yet
impl ComposeOptionsBuilder {
    /// Create a new builder with required components
    ///
    /// Default values:
    /// - transport: "http"
    /// - output: "server.wasm"
    /// - version: current package version
    /// - deps_dir: will be resolved from config
    /// - skip_download: false
    /// - force: false
    pub fn new(components: Vec<String>) -> Self {
        Self {
            components,
            transport: "http".to_string(),
            output: PathBuf::from("server.wasm"),
            version: env!("CARGO_PKG_VERSION").to_string(),
            override_transport: None,
            override_method_not_found: None,
            deps_dir: None,
            skip_download: false,
            force: false,
            verbose: false,
        }
    }

    /// Set the transport type ("http" or "stdio")
    pub fn transport(mut self, transport: impl Into<String>) -> Self {
        self.transport = transport.into();
        self
    }

    /// Set the output path for the composed component
    pub fn output(mut self, output: PathBuf) -> Self {
        self.output = output;
        self
    }

    /// Set the wasmcp version for framework components
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Override the transport component with a custom spec
    pub fn override_transport(mut self, spec: impl Into<String>) -> Self {
        self.override_transport = Some(spec.into());
        self
    }

    /// Override the method-not-found component with a custom spec
    pub fn override_method_not_found(mut self, spec: impl Into<String>) -> Self {
        self.override_method_not_found = Some(spec.into());
        self
    }

    /// Set the directory for downloaded dependencies
    pub fn deps_dir(mut self, deps_dir: PathBuf) -> Self {
        self.deps_dir = Some(deps_dir);
        self
    }

    /// Skip downloading dependencies (use existing files only)
    pub fn skip_download(mut self, skip: bool) -> Self {
        self.skip_download = skip;
        self
    }

    /// Force overwrite of existing output file
    pub fn force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    /// Enable verbose output
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Build the ComposeOptions, resolving deps_dir from config if not set
    pub fn build(self) -> Result<ComposeOptions> {
        let deps_dir = match self.deps_dir {
            Some(dir) => dir,
            None => config::get_deps_dir()
                .context("Failed to get dependencies directory from config")?,
        };

        Ok(ComposeOptions {
            components: self.components,
            transport: self.transport,
            output: self.output,
            version: self.version,
            override_transport: self.override_transport,
            override_method_not_found: self.override_method_not_found,
            deps_dir,
            skip_download: self.skip_download,
            force: self.force,
            verbose: self.verbose,
        })
    }
}

/// Valid transport types supported by the compose command
const VALID_TRANSPORTS: &[&str] = &["http", "stdio"];

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
        version,
        override_transport,
        override_method_not_found,
        deps_dir,
        skip_download,
        force,
        verbose,
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
             example: wasmcp compose my-handler.wasm namespace:other-handler@1.0.0"
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
        &version,
        &deps_dir,
        &client,
        skip_download,
        verbose,
    )
    .await?;

    // Resolve method-not-found component
    let method_not_found_path = resolve_method_not_found_component(
        override_method_not_found.as_deref(),
        &version,
        &deps_dir,
        &client,
        skip_download,
        verbose,
    )
    .await?;

    // Auto-detect and wrap capability components (tools, resources, etc.)
    if verbose {
        println!("\nDetecting component types...");
    }
    let wrapped_components =
        wrapping::wrap_capabilities(component_paths, &deps_dir, &version, verbose).await?;

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
        &transport,
        &version,
        verbose,
    )
    .await?;

    // Write output file
    std::fs::write(&output_path, bytes)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

    // Print success message
    print_success_message(&output_path, &transport);

    Ok(())
}

/// Validate transport type is supported
fn validate_transport(transport: &str) -> Result<()> {
    if !VALID_TRANSPORTS.contains(&transport) {
        anyhow::bail!(
            "unsupported transport type: '{}', must be one of: {}",
            transport,
            VALID_TRANSPORTS.join(", ")
        );
    }
    Ok(())
}

/// Resolve output path - make absolute if relative (using current working directory)
fn resolve_output_path(output: &PathBuf) -> Result<PathBuf> {
    if output.is_absolute() {
        Ok(output.clone())
    } else {
        // Resolve relative paths against current working directory
        let cwd = std::env::current_dir().context("Failed to get current working directory")?;
        Ok(cwd.join(output))
    }
}

/// Validate output file doesn't exist (unless force is set)
fn validate_output_file(output_path: &Path, force: bool) -> Result<()> {
    if output_path.exists() && !force {
        anyhow::bail!(
            "output file '{}' already exists, use --force to overwrite",
            output_path.display()
        );
    }
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
        let path = resolution::resolve_component_spec(spec, deps_dir, client).await?;
        if verbose {
            println!("   {}. {} → {}", i + 1, spec, path.display());
        }
        paths.push(path);
    }

    Ok(paths)
}

/// Framework component type for resolution
enum FrameworkComponent<'a> {
    /// Transport component (e.g., "http-transport", "stdio-transport")
    Transport(&'a str),
    /// Method-not-found terminal handler
    MethodNotFound,
}

impl FrameworkComponent<'_> {
    /// Get the component name for dependency lookup
    fn component_name(&self) -> String {
        match self {
            Self::Transport(transport) => format!("{}-transport", transport),
            Self::MethodNotFound => "method-not-found".to_string(),
        }
    }

    /// Get a human-readable display name
    fn display_name(&self) -> &str {
        match self {
            Self::Transport(_) => "transport",
            Self::MethodNotFound => "method-not-found",
        }
    }

    /// Download dependencies if needed for this component
    async fn ensure_downloaded(
        &self,
        version: &str,
        deps_dir: &Path,
        client: &PackageClient,
        skip_download: bool,
        verbose: bool,
    ) -> Result<()> {
        if skip_download {
            return Ok(());
        }

        match self {
            Self::Transport(transport) => {
                if verbose {
                    println!("\nDownloading framework dependencies...");
                }
                dependencies::download_dependencies(transport, version, deps_dir, client).await
            }
            Self::MethodNotFound => {
                // Check if already exists (transport download includes it)
                let pkg = dependencies::interfaces::package("method-not-found", version);
                let filename = pkg.replace([':', '/'], "_") + ".wasm";
                let path = deps_dir.join(&filename);
                if !path.exists() {
                    dependencies::download_dependencies("http", version, deps_dir, client).await?;
                }
                Ok(())
            }
        }
    }
}

/// Generic resolver for framework components (transport or method-not-found)
async fn resolve_framework_component(
    component: FrameworkComponent<'_>,
    override_spec: Option<&str>,
    version: &str,
    deps_dir: &Path,
    client: &PackageClient,
    skip_download: bool,
    verbose: bool,
) -> Result<PathBuf> {
    if let Some(spec) = override_spec {
        if verbose {
            println!("\nUsing override {}: {}", component.display_name(), spec);
        }
        resolution::resolve_component_spec(spec, deps_dir, client).await
    } else {
        component
            .ensure_downloaded(version, deps_dir, client, skip_download, verbose)
            .await?;
        dependencies::get_dependency_path(&component.component_name(), version, deps_dir)
    }
}

/// Resolve transport component (override or default)
async fn resolve_transport_component(
    transport: &str,
    override_spec: Option<&str>,
    version: &str,
    deps_dir: &Path,
    client: &PackageClient,
    skip_download: bool,
    verbose: bool,
) -> Result<PathBuf> {
    resolve_framework_component(
        FrameworkComponent::Transport(transport),
        override_spec,
        version,
        deps_dir,
        client,
        skip_download,
        verbose,
    )
    .await
}

/// Resolve method-not-found component (override or default)
async fn resolve_method_not_found_component(
    override_spec: Option<&str>,
    version: &str,
    deps_dir: &Path,
    client: &PackageClient,
    skip_download: bool,
    verbose: bool,
) -> Result<PathBuf> {
    resolve_framework_component(
        FrameworkComponent::MethodNotFound,
        override_spec,
        version,
        deps_dir,
        client,
        skip_download,
        verbose,
    )
    .await
}

/// Print the composition pipeline diagram
fn print_pipeline_diagram(transport: &str, components: &[PathBuf]) {
    println!("\nComposing MCP server pipeline...");
    println!("   {} (transport)", transport);
    for (i, path) in components.iter().enumerate() {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("component");
        println!("   ↓");
        println!("   {}. {}", i + 1, name);
    }
    println!("   ↓");
    println!("   method-not-found (terminal handler)");
}

/// Print success message with run instructions
fn print_success_message(output_path: &Path, transport: &str) {
    println!("\nComposed: {}", output_path.display());
    println!("\nTo run the server:");
    match transport {
        "http" => println!("  wasmtime serve -Scli {}", output_path.display()),
        "stdio" => println!("  wasmtime run {}", output_path.display()),
        _ => println!("  wasmtime {}", output_path.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_interface_names() {
        assert_eq!(
            dependencies::interfaces::server_handler("0.1.0-beta.2"),
            "wasmcp:server/handler@0.1.0-beta.2"
        );
        assert_eq!(
            dependencies::interfaces::tools("0.1.0-beta.2"),
            "wasmcp:protocol/tools@0.1.0-beta.2"
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
            dependencies::interfaces::package("http-transport", "0.1.0-beta.2"),
            "wasmcp:http-transport@0.1.0-beta.2"
        );
        assert_eq!(
            dependencies::interfaces::package("method-not-found", "0.1.0-beta.2"),
            "wasmcp:method-not-found@0.1.0-beta.2"
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

    #[test]
    fn test_validate_output_file_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("nonexistent.wasm");
        let result = validate_output_file(&path, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_output_file_exists_without_force() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let result = validate_output_file(temp_file.path(), false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("already exists"));
        assert!(err_msg.contains("--force"));
    }

    #[test]
    fn test_validate_output_file_exists_with_force() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let result = validate_output_file(temp_file.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compose_options_builder() {
        let options = ComposeOptionsBuilder::new(vec!["handler.wasm".to_string()])
            .transport("stdio")
            .force(true)
            .skip_download(true)
            .build()
            .unwrap();

        assert_eq!(options.components, vec!["handler.wasm"]);
        assert_eq!(options.transport, "stdio");
        assert!(options.force);
        assert!(options.skip_download);
    }

    #[test]
    fn test_compose_options_builder_defaults() {
        let options = ComposeOptionsBuilder::new(vec!["handler.wasm".to_string()])
            .build()
            .unwrap();

        assert_eq!(options.transport, "http");
        assert_eq!(options.output, PathBuf::from("server.wasm"));
        assert!(!options.force);
        assert!(!options.skip_download);
        assert_eq!(options.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_compose_options_builder_chaining() {
        let options = ComposeOptionsBuilder::new(vec!["a.wasm".to_string()])
            .transport("http")
            .output(PathBuf::from("out.wasm"))
            .version("1.0.0")
            .override_transport("custom-transport.wasm")
            .override_method_not_found("custom-mnf.wasm")
            .build()
            .unwrap();

        assert_eq!(options.version, "1.0.0");
        assert_eq!(
            options.override_transport,
            Some("custom-transport.wasm".to_string())
        );
        assert_eq!(
            options.override_method_not_found,
            Some("custom-mnf.wasm".to_string())
        );
    }

    #[test]
    fn test_validate_transport_http() {
        let result = validate_transport("http");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_transport_stdio() {
        let result = validate_transport("stdio");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_transport_invalid() {
        let result = validate_transport("websocket");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("unsupported transport type"));
        assert!(err_msg.contains("websocket"));
        assert!(err_msg.contains("http"));
        assert!(err_msg.contains("stdio"));
    }
}
