//! Framework component resolution
//!
//! This module handles resolution and downloading of wasmcp framework components
//! (transport, method-not-found, http-notifications). Framework components are
//! downloaded from OCI registries and cached locally.
//!
//! # Framework Components
//!
//! - **Transport**: HTTP or stdio server wrapper (`http-transport`, `stdio-transport`)
//! - **MethodNotFound**: Terminal handler that returns errors for unknown methods
//! - **HttpNotifications**: Progress/logging support for HTTP transport
//!
//! # Resolution Flow
//!
//! 1. Check if override spec provided (custom component)
//! 2. If no override, ensure framework component is downloaded
//! 3. Return path to local component file
//!
//! # Examples
//!
//! ```rust,ignore
//! # use wasmcp::commands::compose::framework::FrameworkComponent;
//! let transport = FrameworkComponent::Transport("http");
//! assert_eq!(transport.component_name(), "http-transport");
//! assert_eq!(transport.display_name(), "transport");
//! ```

use anyhow::Result;
use std::path::{Path, PathBuf};

use super::{PackageClient, dependencies, resolution};
use crate::versioning::VersionResolver;

/// Framework component type for resolution
///
/// Represents the different types of framework components that wasmcp
/// automatically provides for server composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameworkComponent<'a> {
    /// Transport component (e.g., "http-transport", "stdio-transport")
    ///
    /// Wraps the server with WASI HTTP or CLI interfaces.
    Transport(&'a str),

    /// Method-not-found terminal handler
    ///
    /// Returns proper MCP errors for unimplemented methods.
    MethodNotFound,

    /// HTTP notifications provider
    ///
    /// Adds progress/log notification support for HTTP servers.
    HttpNotifications,
}

impl FrameworkComponent<'_> {
    /// Get the component name for dependency lookup
    ///
    /// Transforms the framework component type into the package name
    /// used in OCI registries (e.g., `http-transport`, `method-not-found`).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # use wasmcp::commands::compose::framework::FrameworkComponent;
    /// let http = FrameworkComponent::Transport("http");
    /// assert_eq!(http.component_name(), "http-transport");
    ///
    /// let mnf = FrameworkComponent::MethodNotFound;
    /// assert_eq!(mnf.component_name(), "method-not-found");
    /// ```
    pub fn component_name(&self) -> String {
        match self {
            Self::Transport(transport) => format!("{}-transport", transport),
            Self::MethodNotFound => "method-not-found".to_string(),
            Self::HttpNotifications => "http-notifications".to_string(),
        }
    }

    /// Get a human-readable display name
    ///
    /// Returns a simplified name for user-facing messages.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # use wasmcp::commands::compose::framework::FrameworkComponent;
    /// let http = FrameworkComponent::Transport("http");
    /// assert_eq!(http.display_name(), "transport");
    ///
    /// let mnf = FrameworkComponent::MethodNotFound;
    /// assert_eq!(mnf.display_name(), "method-not-found");
    /// ```
    pub fn display_name(&self) -> &str {
        match self {
            Self::Transport(_) => "transport",
            Self::MethodNotFound => "method-not-found",
            Self::HttpNotifications => "http-notifications",
        }
    }

    /// Download dependencies if needed for this component
    ///
    /// Ensures the framework component is available locally by downloading
    /// it from the registry if not already cached.
    ///
    /// # Arguments
    ///
    /// * `resolver` - Version resolver for component versions
    /// * `deps_dir` - Directory where dependencies are cached
    /// * `client` - OCI package client for downloads
    /// * `skip_download` - If true, assume components already exist
    /// * `verbose` - Show download progress messages
    ///
    /// # Errors
    ///
    /// Returns an error if download fails or component cannot be found.
    pub async fn ensure_downloaded(
        &self,
        resolver: &VersionResolver,
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
                dependencies::download_dependencies(transport, resolver, deps_dir, client).await
            }
            Self::MethodNotFound | Self::HttpNotifications => {
                // Check if already exists (transport download includes it)
                let version = resolver.get_version(&self.component_name())?;
                let pkg =
                    dependencies::interfaces::package(self.component_name().as_str(), &version);
                let filename = pkg.replace([':', '/'], "_") + ".wasm";
                let path = deps_dir.join(&filename);
                if !path.exists() {
                    dependencies::download_dependencies("http", resolver, deps_dir, client).await?;
                }
                Ok(())
            }
        }
    }
}

/// Generic resolver for framework components (transport or method-not-found)
///
/// Resolves a framework component to a local file path, either by using an
/// override spec or by downloading the default framework component.
///
/// # Arguments
///
/// * `component` - Type of framework component to resolve
/// * `override_spec` - Optional custom component spec to use instead of default
/// * `resolver` - Version resolver for component versions
/// * `deps_dir` - Directory where dependencies are cached
/// * `client` - OCI package client for downloads
/// * `skip_download` - If true, assume components already exist
/// * `verbose` - Show resolution messages
///
/// # Returns
///
/// Path to the resolved component file.
///
/// # Errors
///
/// Returns an error if:
/// - Override spec cannot be resolved
/// - Framework component download fails
/// - Component file not found after download
pub async fn resolve_framework_component(
    component: FrameworkComponent<'_>,
    override_spec: Option<&str>,
    resolver: &VersionResolver,
    deps_dir: &Path,
    client: &PackageClient,
    skip_download: bool,
    verbose: bool,
) -> Result<PathBuf> {
    if let Some(spec) = override_spec {
        if verbose {
            println!("\nUsing override {}: {}", component.display_name(), spec);
        }
        resolution::resolve_component_spec(spec, deps_dir, client, verbose).await
    } else {
        component
            .ensure_downloaded(resolver, deps_dir, client, skip_download, verbose)
            .await?;
        dependencies::get_dependency_path(&component.component_name(), resolver, deps_dir)
    }
}

/// Resolve transport component (override or default)
///
/// Convenience wrapper for resolving transport components specifically.
///
/// # Examples
///
/// ```rust,ignore
/// # use wasmcp::commands::compose::framework::resolve_transport_component;
/// # use wasmcp::commands::compose::PackageClient;
/// # use std::path::Path;
/// # async fn example() -> anyhow::Result<()> {
/// let client = PackageClient::new_with_default_config()?;
/// let deps_dir = Path::new("~/.config/wasmcp/deps");
///
/// // Resolve default HTTP transport
/// let path = resolve_transport_component(
///     "http",
///     None,
///     "0.1.0",
///     deps_dir,
///     &client,
///     false,
///     true
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn resolve_transport_component(
    transport: &str,
    override_spec: Option<&str>,
    resolver: &VersionResolver,
    deps_dir: &Path,
    client: &PackageClient,
    skip_download: bool,
    verbose: bool,
) -> Result<PathBuf> {
    resolve_framework_component(
        FrameworkComponent::Transport(transport),
        override_spec,
        resolver,
        deps_dir,
        client,
        skip_download,
        verbose,
    )
    .await
}

/// Resolve method-not-found component (override or default)
///
/// Convenience wrapper for resolving the method-not-found terminal handler.
pub async fn resolve_method_not_found_component(
    override_spec: Option<&str>,
    resolver: &VersionResolver,
    deps_dir: &Path,
    client: &PackageClient,
    skip_download: bool,
    verbose: bool,
) -> Result<PathBuf> {
    resolve_framework_component(
        FrameworkComponent::MethodNotFound,
        override_spec,
        resolver,
        deps_dir,
        client,
        skip_download,
        verbose,
    )
    .await
}

/// Resolve http-notifications component (default only, no override)
///
/// Convenience wrapper for resolving the HTTP notifications component.
/// This component does not support overrides as it's tightly coupled
/// to the HTTP transport implementation.
pub async fn resolve_http_notifications_component(
    resolver: &VersionResolver,
    deps_dir: &Path,
    client: &PackageClient,
    skip_download: bool,
    verbose: bool,
) -> Result<PathBuf> {
    resolve_framework_component(
        FrameworkComponent::HttpNotifications,
        None,
        resolver,
        deps_dir,
        client,
        skip_download,
        verbose,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test FrameworkComponent::component_name()
    #[test]
    fn test_framework_component_naming() {
        let http_transport = FrameworkComponent::Transport("http");
        assert_eq!(http_transport.component_name(), "http-transport");

        let stdio_transport = FrameworkComponent::Transport("stdio");
        assert_eq!(stdio_transport.component_name(), "stdio-transport");

        let mnf = FrameworkComponent::MethodNotFound;
        assert_eq!(mnf.component_name(), "method-not-found");

        let notifications = FrameworkComponent::HttpNotifications;
        assert_eq!(notifications.component_name(), "http-notifications");
    }

    /// Test FrameworkComponent::display_name()
    #[test]
    fn test_framework_component_display_names() {
        let http_transport = FrameworkComponent::Transport("http");
        assert_eq!(http_transport.display_name(), "transport");

        let mnf = FrameworkComponent::MethodNotFound;
        assert_eq!(mnf.display_name(), "method-not-found");

        let notifications = FrameworkComponent::HttpNotifications;
        assert_eq!(notifications.display_name(), "http-notifications");
    }

    /// Test framework download message format
    #[test]
    fn test_framework_download_message() {
        let message = "Downloading framework dependencies...";
        assert!(message.contains("framework"));
        assert!(message.contains("dependencies"));
    }
}
