//! Framework component resolution
//!
//! This module handles resolution and downloading of wasmcp framework components
//! (transport, method-not-found, http-messages). Framework components are
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
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::{PackageClient, dependencies, resolution, wrapping::SessionsDraft};
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

    /// HTTP messages provider
    ///
    /// Adds progress/log notification support for HTTP servers.
    Httpmessages,

    /// Sessions provider
    ///
    /// Provides MCP session management backed by WASI KV.
    Sessions,

    /// Session store provider
    ///
    /// Provides centralized session storage with session-manager interface.
    SessionStore,
}

impl FrameworkComponent<'_> {
    /// Get the component name for dependency lookup
    ///
    /// Transforms the framework component type into the package name
    /// used in OCI registries. Only runtime-specific components (sessions, session-store)
    /// that import wasi:keyvalue get `-d2` suffix for draft2 variants.
    ///
    /// Transport components are now runtime-agnostic since they use session-manager
    /// instead of wasi:keyvalue directly.
    ///
    /// # Arguments
    ///
    /// * `draft` - The WASI draft version to use for runtime-specific components
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # use wasmcp::commands::compose::framework::FrameworkComponent;
    /// # use wasmcp::commands::compose::wrapping::SessionsDraft;
    /// let http = FrameworkComponent::Transport("http");
    /// assert_eq!(http.component_name(SessionsDraft::Draft), "http-transport");
    /// assert_eq!(http.component_name(SessionsDraft::Draft2), "http-transport");  // Same - runtime-agnostic
    ///
    /// let sessions = FrameworkComponent::Sessions;
    /// assert_eq!(sessions.component_name(SessionsDraft::Draft), "sessions");
    /// assert_eq!(sessions.component_name(SessionsDraft::Draft2), "sessions-d2");
    ///
    /// let mnf = FrameworkComponent::MethodNotFound;
    /// assert_eq!(mnf.component_name(SessionsDraft::Draft), "method-not-found");
    /// ```
    pub fn component_name(&self, draft: SessionsDraft) -> String {
        let suffix = match draft {
            SessionsDraft::Draft => "",
            SessionsDraft::Draft2 => "-d2",
        };

        match self {
            // Runtime-agnostic components (no suffix) - includes transports now
            Self::Transport(transport) => format!("{}-transport", transport),
            Self::MethodNotFound => "method-not-found".to_string(),
            Self::Httpmessages => "http-messages".to_string(),

            // Runtime-specific components (wasi:keyvalue) get draft suffix
            Self::Sessions => format!("sessions{}", suffix),
            Self::SessionStore => format!("session-store{}", suffix),
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
            Self::Httpmessages => "http-messages",
            Self::Sessions => "sessions",
            Self::SessionStore => "session-store",
        }
    }

    /// Download dependencies if needed for this component
    ///
    /// Ensures the framework component is available locally by downloading
    /// it from the registry if not already cached.
    ///
    /// # Arguments
    ///
    /// * `draft` - WASI draft version for runtime-specific components
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
        draft: SessionsDraft,
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
                // Download all core components (no optionals yet - caller handles detection)
                let optional_components = HashSet::new();
                dependencies::download_dependencies(transport, draft, &optional_components, resolver, deps_dir, client).await
            }
            Self::MethodNotFound | Self::Httpmessages | Self::Sessions | Self::SessionStore => {
                // Check if already exists (transport download includes it)
                let version = resolver.get_version(&self.component_name(draft))?;
                let pkg =
                    dependencies::interfaces::package(self.component_name(draft).as_str(), &version);
                let filename = pkg.replace([':', '/'], "_") + ".wasm";
                let path = deps_dir.join(&filename);
                if !path.exists() {
                    // Component not found - need to download with appropriate optionals
                    let mut optional_components = HashSet::new();
                    if matches!(self, Self::Httpmessages) {
                        optional_components.insert(dependencies::OptionalComponent::HttpMessages);
                    }
                    if matches!(self, Self::Sessions) {
                        optional_components.insert(dependencies::OptionalComponent::Sessions);
                    }
                    if matches!(self, Self::SessionStore) {
                        optional_components.insert(dependencies::OptionalComponent::SessionStore);
                    }
                    dependencies::download_dependencies("http", draft, &optional_components, resolver, deps_dir, client).await?;
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
/// * `draft` - WASI draft version for runtime-specific components
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
    draft: SessionsDraft,
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
            .ensure_downloaded(draft, resolver, deps_dir, client, skip_download, verbose)
            .await?;
        dependencies::get_dependency_path(&component.component_name(draft), resolver, deps_dir)
    }
}

/// Resolve transport component (override or default)
///
/// Convenience wrapper for resolving transport components specifically.
pub async fn resolve_transport_component(
    transport: &str,
    draft: SessionsDraft,
    override_spec: Option<&str>,
    resolver: &VersionResolver,
    deps_dir: &Path,
    client: &PackageClient,
    skip_download: bool,
    verbose: bool,
) -> Result<PathBuf> {
    resolve_framework_component(
        FrameworkComponent::Transport(transport),
        draft,
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
    draft: SessionsDraft,
    override_spec: Option<&str>,
    resolver: &VersionResolver,
    deps_dir: &Path,
    client: &PackageClient,
    skip_download: bool,
    verbose: bool,
) -> Result<PathBuf> {
    resolve_framework_component(
        FrameworkComponent::MethodNotFound,
        draft,
        override_spec,
        resolver,
        deps_dir,
        client,
        skip_download,
        verbose,
    )
    .await
}

/// Resolve http-messages component (default only, no override)
///
/// Convenience wrapper for resolving the HTTP messages component.
/// This component does not support overrides as it's tightly coupled
/// to the HTTP transport implementation.
pub async fn resolve_http_messages_component(
    draft: SessionsDraft,
    resolver: &VersionResolver,
    deps_dir: &Path,
    client: &PackageClient,
    skip_download: bool,
    verbose: bool,
) -> Result<PathBuf> {
    resolve_framework_component(
        FrameworkComponent::Httpmessages,
        draft,
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
        use SessionsDraft::*;

        // Transports are runtime-agnostic (no draft suffix)
        let http_transport = FrameworkComponent::Transport("http");
        assert_eq!(http_transport.component_name(Draft), "http-transport");
        assert_eq!(http_transport.component_name(Draft2), "http-transport");

        let stdio_transport = FrameworkComponent::Transport("stdio");
        assert_eq!(stdio_transport.component_name(Draft), "stdio-transport");
        assert_eq!(stdio_transport.component_name(Draft2), "stdio-transport");

        // Method-not-found is runtime-agnostic
        let mnf = FrameworkComponent::MethodNotFound;
        assert_eq!(mnf.component_name(Draft), "method-not-found");
        assert_eq!(mnf.component_name(Draft2), "method-not-found");

        // HTTP messages is runtime-agnostic
        let messages = FrameworkComponent::Httpmessages;
        assert_eq!(messages.component_name(Draft), "http-messages");
        assert_eq!(messages.component_name(Draft2), "http-messages");

        // Sessions are runtime-specific (wasi:keyvalue) - get draft suffix
        let sessions = FrameworkComponent::Sessions;
        assert_eq!(sessions.component_name(Draft), "sessions");
        assert_eq!(sessions.component_name(Draft2), "sessions-d2");

        // Session-store is runtime-specific (wasi:keyvalue) - get draft suffix
        let session_store = FrameworkComponent::SessionStore;
        assert_eq!(session_store.component_name(Draft), "session-store");
        assert_eq!(session_store.component_name(Draft2), "session-store-d2");
    }

    /// Test FrameworkComponent::display_name()
    #[test]
    fn test_framework_component_display_names() {
        let http_transport = FrameworkComponent::Transport("http");
        assert_eq!(http_transport.display_name(), "transport");

        let mnf = FrameworkComponent::MethodNotFound;
        assert_eq!(mnf.display_name(), "method-not-found");

        let messages = FrameworkComponent::Httpmessages;
        assert_eq!(messages.display_name(), "http-messages");

        let sessions = FrameworkComponent::Sessions;
        assert_eq!(sessions.display_name(), "sessions");
    }

    /// Test framework download message format
    #[test]
    fn test_framework_download_message() {
        let message = "Downloading framework dependencies...";
        assert!(message.contains("framework"));
        assert!(message.contains("dependencies"));
    }
}

/// Download all framework dependencies in one batch
///
/// Detects what's needed and downloads everything at once.
pub async fn download_framework_dependencies(
    transport: &str,
    draft: SessionsDraft,
    sessions_needed: bool,
    session_store_needed: bool,
    http_messages_needed: bool,
    resolver: &VersionResolver,
    deps_dir: &Path,
    client: &PackageClient,
) -> Result<()> {
    // Build set of optional components
    let mut optional_components = HashSet::new();
    if sessions_needed {
        optional_components.insert(dependencies::OptionalComponent::Sessions);
    }
    if session_store_needed {
        optional_components.insert(dependencies::OptionalComponent::SessionStore);
    }
    if http_messages_needed {
        optional_components.insert(dependencies::OptionalComponent::HttpMessages);
    }

    dependencies::download_dependencies(transport, draft, &optional_components, resolver, deps_dir, client).await
}

/// Resolve component path (override or cached)
///
/// Simple path resolution - download should happen separately.
pub async fn resolve_component_path(
    component_name: &str,
    draft: SessionsDraft,
    override_spec: Option<&str>,
    resolver: &VersionResolver,
    deps_dir: &Path,
    client: &PackageClient,
    verbose: bool,
) -> Result<PathBuf> {
    if let Some(spec) = override_spec {
        if verbose {
            println!("\nUsing override for {}: {}", component_name, spec);
        }
        resolution::resolve_component_spec(spec, deps_dir, client, verbose).await
    } else {
        // Apply draft suffix ONLY for runtime-specific components (wasi:keyvalue)
        // Transports are now runtime-agnostic (use session-manager, not wasi:keyvalue)
        let name_with_draft = if component_name == "sessions" || component_name == "session-store" {
            match draft {
                SessionsDraft::Draft => component_name.to_string(),
                SessionsDraft::Draft2 => format!("{}-d2", component_name),
            }
        } else {
            component_name.to_string()
        };
        dependencies::get_dependency_path(&name_with_draft, resolver, deps_dir)
    }
}
