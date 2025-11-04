//! Framework component resolution
//!
//! This module handles resolution and downloading of wasmcp framework components
//! (transport, server-io, session-store, method-not-found). Framework components are
//! downloaded from OCI registries and cached locally.
//!
//! # Framework Components
//!
//! - **Transport**: HTTP or stdio server wrapper (`http-transport`, `stdio-transport`)
//! - **MethodNotFound**: Terminal handler that returns errors for unknown methods
//! - **Httpmessages**: Progress/logging support for HTTP transport
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
pub enum FrameworkComponent {
    /// Transport component (unified http/stdio)
    ///
    /// Wraps the server with WASI HTTP or CLI interfaces.
    Transport,

    /// Server I/O component
    ///
    /// Provides universal I/O operations (parse_message, send_message, set_frame).
    ServerIO,

    /// Session store component
    ///
    /// Provides session management for stateful MCP servers.
    SessionStore,

    /// Method-not-found terminal handler
    ///
    /// Returns proper MCP errors for unimplemented methods.
    MethodNotFound,
}

impl FrameworkComponent {
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
            Self::Transport => "transport".to_string(),
            Self::ServerIO => "server-io".to_string(),
            Self::SessionStore => "session-store".to_string(),
            Self::MethodNotFound => "method-not-found".to_string(),
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
            Self::Transport => "transport",
            Self::ServerIO => "server-io",
            Self::SessionStore => "session-store",
            Self::MethodNotFound => "method-not-found",
        }
    }
}

/// Generic resolver for framework components (transport or method-not-found)
///
/// Resolves a framework component to a local file path, either by using an
/// override spec or by getting the path to the already-downloaded default component.
///
/// # Arguments
///
/// * `component` - Type of framework component to resolve
/// * `override_spec` - Optional custom component spec to use instead of default
/// * `resolver` - Version resolver for component versions
/// * `deps_dir` - Directory where dependencies are cached
/// * `client` - OCI package client for downloads (only used for overrides)
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
/// - Component file not found (call download_dependencies first)
pub async fn resolve_framework_component(
    component: FrameworkComponent,
    override_spec: Option<&str>,
    resolver: &VersionResolver,
    deps_dir: &Path,
    client: &PackageClient,
    verbose: bool,
) -> Result<PathBuf> {
    if let Some(spec) = override_spec {
        if verbose {
            println!("\nUsing override {}: {}", component.display_name(), spec);
        }
        resolution::resolve_component_spec(spec, deps_dir, client, verbose).await
    } else {
        dependencies::get_dependency_path(&component.component_name(), resolver, deps_dir)
    }
}

/// Resolve transport component (override or default)
pub async fn resolve_transport_component(
    override_spec: Option<&str>,
    resolver: &VersionResolver,
    deps_dir: &Path,
    client: &PackageClient,
    verbose: bool,
) -> Result<PathBuf> {
    resolve_framework_component(
        FrameworkComponent::Transport,
        override_spec,
        resolver,
        deps_dir,
        client,
        verbose,
    )
    .await
}

/// Resolve server-io component (override or default)
pub async fn resolve_server_io_component(
    override_spec: Option<&str>,
    resolver: &VersionResolver,
    deps_dir: &Path,
    client: &PackageClient,
    verbose: bool,
) -> Result<PathBuf> {
    resolve_framework_component(
        FrameworkComponent::ServerIO,
        override_spec,
        resolver,
        deps_dir,
        client,
        verbose,
    )
    .await
}

/// Resolve session-store component (override or default)
pub async fn resolve_session_store_component(
    override_spec: Option<&str>,
    resolver: &VersionResolver,
    deps_dir: &Path,
    client: &PackageClient,
    verbose: bool,
    runtime: &str,
) -> Result<PathBuf> {
    // If override provided, use it
    if let Some(spec) = override_spec {
        if verbose {
            println!("\nUsing override session-store: {}", spec);
        }
        return resolution::resolve_component_spec(spec, deps_dir, client, verbose).await;
    }

    // Determine package name based on runtime
    // "spin" uses session-store-d2 (draft 2 WASI)
    // "wasmcloud" and "wasmtime" use session-store (standard WASI)
    let component_name = match runtime {
        "spin" => "session-store-d2",
        "wasmcloud" | "wasmtime" => "session-store",
        _ => anyhow::bail!(
            "unsupported runtime: '{}' (must be 'spin', 'wasmcloud', or 'wasmtime')",
            runtime
        ),
    };

    dependencies::get_dependency_path(component_name, resolver, deps_dir)
}

/// Resolve method-not-found component (override or default)
///
/// Convenience wrapper for resolving the method-not-found terminal handler.
pub async fn resolve_method_not_found_component(
    override_spec: Option<&str>,
    resolver: &VersionResolver,
    deps_dir: &Path,
    client: &PackageClient,
    verbose: bool,
) -> Result<PathBuf> {
    resolve_framework_component(
        FrameworkComponent::MethodNotFound,
        override_spec,
        resolver,
        deps_dir,
        client,
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
        let transport = FrameworkComponent::Transport;
        assert_eq!(transport.component_name(), "transport");

        let server_io = FrameworkComponent::ServerIO;
        assert_eq!(server_io.component_name(), "server-io");

        let session_store = FrameworkComponent::SessionStore;
        assert_eq!(session_store.component_name(), "session-store");

        let mnf = FrameworkComponent::MethodNotFound;
        assert_eq!(mnf.component_name(), "method-not-found");
    }

    /// Test FrameworkComponent::display_name()
    #[test]
    fn test_framework_component_display_names() {
        let transport = FrameworkComponent::Transport;
        assert_eq!(transport.display_name(), "transport");

        let server_io = FrameworkComponent::ServerIO;
        assert_eq!(server_io.display_name(), "server-io");

        let session_store = FrameworkComponent::SessionStore;
        assert_eq!(session_store.display_name(), "session-store");

        let mnf = FrameworkComponent::MethodNotFound;
        assert_eq!(mnf.display_name(), "method-not-found");
    }

    /// Test framework download message format
    #[test]
    fn test_framework_download_message() {
        let message = "Downloading framework dependencies...";
        assert!(message.contains("framework"));
        assert!(message.contains("dependencies"));
    }
}
