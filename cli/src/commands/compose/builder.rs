//! Builder pattern for ComposeOptions
//!
//! Provides a fluent API for constructing [`ComposeOptions`] with sensible defaults.
//! This builder is part of the public API for programmatic use and is particularly
//! useful for external consumers who want to compose components programmatically.
//!
//! # Examples
//!
//! ```rust
//! use wasmcp::commands::compose::ComposeOptionsBuilder;
//! use std::path::PathBuf;
//!
//! # fn example() -> anyhow::Result<()> {
//! let options = ComposeOptionsBuilder::new(vec!["./handler.wasm".to_string()])
//!     .transport("stdio")
//!     .output(PathBuf::from("my-server.wasm"))
//!     .force(true)
//!     .build()?;
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use std::path::PathBuf;

use super::{ComposeOptions, CompositionMode};
use crate::config;
use crate::versioning::VersionResolver;

/// Builder for ComposeOptions with sensible defaults
///
/// This builder provides a fluent API for constructing composition options.
/// All fields have sensible defaults except `components` which must be provided.
///
/// # Default Values
///
/// - `transport`: "http"
/// - `output`: "server.wasm"
/// - `deps_dir`: Resolved from config (XDG directories)
/// - `skip_download`: false
/// - `force`: false
/// - `verbose`: false
/// - `mode`: Server (complete MCP server)
#[derive(Debug, Clone)]
#[allow(dead_code)] // Public API - used by external consumers, not internally yet
pub struct ComposeOptionsBuilder {
    components: Vec<String>,
    transport: String,
    output: PathBuf,
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
    /// The components list specifies which middleware/handler components to include
    /// in the composition. Each component can be:
    /// - A local file path (relative or absolute)
    /// - A package spec (e.g., "namespace:package@version")
    /// - A registry alias (configured in wasmcp config)
    ///
    /// # Arguments
    ///
    /// * `components` - List of component specs to compose
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use wasmcp::commands::compose::ComposeOptionsBuilder;
    /// let builder = ComposeOptionsBuilder::new(vec![
    ///     "./local-handler.wasm".to_string(),
    ///     "wasmcp:calculator@0.1.0".to_string(),
    /// ]);
    /// ```
    pub fn new(components: Vec<String>) -> Self {
        Self {
            components,
            transport: "http".to_string(),
            output: PathBuf::from("server.wasm"),
            override_transport: None,
            override_method_not_found: None,
            deps_dir: None,
            skip_download: false,
            force: false,
            verbose: false,
        }
    }

    /// Set the transport type ("http" or "stdio")
    ///
    /// - `http`: Creates a server component that runs via `wasmtime serve`
    /// - `stdio`: Creates a CLI component that runs via `wasmtime run`
    pub fn transport(mut self, transport: impl Into<String>) -> Self {
        self.transport = transport.into();
        self
    }

    /// Set the output path for the composed component
    ///
    /// If relative, will be resolved against the current working directory.
    pub fn output(mut self, output: PathBuf) -> Self {
        self.output = output;
        self
    }

    /// Override the transport component with a custom spec
    ///
    /// By default, the transport is downloaded from the wasmcp registry.
    /// This allows using a custom transport implementation.
    pub fn override_transport(mut self, spec: impl Into<String>) -> Self {
        self.override_transport = Some(spec.into());
        self
    }

    /// Override the method-not-found component with a custom spec
    ///
    /// By default, the terminal handler is downloaded from the wasmcp registry.
    /// This allows using a custom terminal handler implementation.
    pub fn override_method_not_found(mut self, spec: impl Into<String>) -> Self {
        self.override_method_not_found = Some(spec.into());
        self
    }

    /// Set the directory for downloaded dependencies
    ///
    /// By default, dependencies are cached in the wasmcp config directory.
    /// This allows using a custom cache location.
    pub fn deps_dir(mut self, deps_dir: PathBuf) -> Self {
        self.deps_dir = Some(deps_dir);
        self
    }

    /// Skip downloading dependencies (use existing files only)
    ///
    /// When true, assumes all framework components are already downloaded.
    /// Useful for offline builds or when dependencies are pre-cached.
    pub fn skip_download(mut self, skip: bool) -> Self {
        self.skip_download = skip;
        self
    }

    /// Force overwrite of existing output file
    ///
    /// By default, composition fails if the output file already exists.
    /// This allows overwriting the output file.
    pub fn force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    /// Enable verbose output
    ///
    /// Shows detailed progress during composition including:
    /// - Component resolution steps
    /// - Component type detection
    /// - Composition pipeline diagram
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Build the ComposeOptions, resolving deps_dir from config if not set
    ///
    /// This consumes the builder and returns a configured [`ComposeOptions`].
    /// The deps_dir will be resolved from XDG config directories if not explicitly set.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Unable to determine the dependencies directory from config
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use wasmcp::commands::compose::ComposeOptionsBuilder;
    /// # fn example() -> anyhow::Result<()> {
    /// let options = ComposeOptionsBuilder::new(vec!["handler.wasm".to_string()])
    ///     .transport("http")
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> Result<ComposeOptions> {
        let deps_dir = match self.deps_dir {
            Some(dir) => dir,
            None => config::get_deps_dir()
                .context("Failed to get dependencies directory from config")?,
        };

        // Create version resolver
        let version_resolver = VersionResolver::new()
            .context("Failed to create version resolver")?;

        Ok(ComposeOptions {
            components: self.components,
            transport: self.transport,
            output: self.output,
            version_resolver,
            override_transport: self.override_transport,
            override_method_not_found: self.override_method_not_found,
            deps_dir,
            skip_download: self.skip_download,
            force: self.force,
            verbose: self.verbose,
            mode: CompositionMode::Server, // Builder defaults to server mode
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Version comes from embedded versions.toml
        assert!(options.version_resolver.get_version("server").is_ok());
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
        assert!(options.version_resolver.get_version("server").is_ok());
        assert_eq!(
            options.override_transport,
            Some("custom-transport.wasm".to_string())
        );
        assert_eq!(
            options.override_method_not_found,
            Some("custom-mnf.wasm".to_string())
        );
    }
}
