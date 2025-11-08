//! Framework dependency management
//!
//! This module handles downloading and locating wasmcp framework components
//! such as transports, method-not-found handler, and tools-middleware.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::commands::pkg;
use crate::versioning::VersionResolver;

use crate::commands::compose::inspection::interfaces;

/// Type alias for the package client used throughout composition
pub type PackageClient =
    wasm_pkg_client::caching::CachingClient<wasm_pkg_client::caching::FileCache>;

/// Configuration for which dependencies to skip downloading
pub struct DownloadConfig<'a> {
    pub skip_transport: bool,
    pub skip_server_io: bool,
    pub skip_kv_store: bool,
    pub skip_session_store: bool,
    pub skip_method_not_found: bool,
    pub skip_tools_middleware: bool,
    pub skip_resources_middleware: bool,
    pub skip_prompts_middleware: bool,
    pub resolver: &'a VersionResolver,
}

impl<'a> DownloadConfig<'a> {
    /// Create config from override options
    pub fn from_overrides(
        resolver: &'a VersionResolver,
        override_transport: Option<&str>,
        override_server_io: Option<&str>,
        override_kv_store: Option<&str>,
        override_session_store: Option<&str>,
        override_method_not_found: Option<&str>,
        override_tools_middleware: Option<&str>,
        override_resources_middleware: Option<&str>,
        override_prompts_middleware: Option<&str>,
    ) -> Self {
        Self {
            skip_transport: override_transport.is_some(),
            skip_server_io: override_server_io.is_some(),
            skip_kv_store: override_kv_store.is_some(),
            skip_session_store: override_session_store.is_some(),
            skip_method_not_found: override_method_not_found.is_some(),
            skip_tools_middleware: override_tools_middleware.is_some(),
            skip_resources_middleware: override_resources_middleware.is_some(),
            skip_prompts_middleware: override_prompts_middleware.is_some(),
            resolver,
        }
    }
}

/// Download required framework dependencies (transport, server-io, kv-store variants, session-store, method-not-found, and all middleware)
///
/// Only downloads components that don't have overrides provided
pub async fn download_dependencies(
    config: &DownloadConfig<'_>,
    deps_dir: &Path,
    client: &PackageClient,
) -> Result<()> {
    let mut specs = Vec::new();

    // Only download components that aren't overridden
    if !config.skip_transport {
        let version = config.resolver.get_version("transport")?;
        specs.push(interfaces::package("transport", &version));
    }

    if !config.skip_server_io {
        let version = config.resolver.get_version("server-io")?;
        specs.push(interfaces::package("server-io", &version));
    }

    if !config.skip_kv_store {
        let version = config.resolver.get_version("kv-store")?;
        specs.push(interfaces::package("kv-store", &version));
        specs.push(interfaces::package("kv-store-d2", &version));
    }

    if !config.skip_session_store {
        let version = config.resolver.get_version("session-store")?;
        specs.push(interfaces::package("session-store", &version));
    }

    if !config.skip_method_not_found {
        let version = config.resolver.get_version("method-not-found")?;
        specs.push(interfaces::package("method-not-found", &version));
    }

    if !config.skip_tools_middleware {
        let version = config.resolver.get_version("tools-middleware")?;
        specs.push(interfaces::package("tools-middleware", &version));
    }

    if !config.skip_resources_middleware {
        let version = config.resolver.get_version("resources-middleware")?;
        specs.push(interfaces::package("resources-middleware", &version));
    }

    if !config.skip_prompts_middleware {
        let version = config.resolver.get_version("prompts-middleware")?;
        specs.push(interfaces::package("prompts-middleware", &version));
    }

    if specs.is_empty() {
        return Ok(());
    }

    pkg::download_packages(client, &specs, deps_dir).await
}

/// Get the file path for a framework dependency
///
/// Framework dependencies are always stored as `wasmcp_{name}@{version}.wasm`
/// Special cases: *-d2 variants use the version from the base component entry
pub fn get_dependency_path(
    name: &str,
    resolver: &VersionResolver,
    deps_dir: &Path,
) -> Result<PathBuf> {
    // *-d2 variants use the same version as their base component
    let version_key = if name == "session-store-d2" {
        "session-store"
    } else if name == "kv-store-d2" {
        "kv-store"
    } else {
        name
    };

    let version = resolver.get_version(version_key)?;
    let filename = format!("wasmcp_{}@{}.wasm", name, version);
    let path = deps_dir.join(&filename);

    if !path.exists() {
        anyhow::bail!(
            "dependency '{}' (version {}) not found at '{}', run without --skip-download",
            name,
            version,
            path.display()
        );
    }

    Ok(path)
}

/// Get the file path for a framework dependency with explicit version
///
/// This variant allows specifying an explicit version, useful when the version
/// is already known or when working with locked versions.
pub fn get_dependency_path_versioned(
    name: &str,
    version: &str,
    deps_dir: &Path,
) -> Result<PathBuf> {
    let filename = format!("wasmcp_{}@{}.wasm", name, version);
    let path = deps_dir.join(&filename);

    if !path.exists() {
        anyhow::bail!(
            "dependency '{}' (version {}) not found at '{}', run without --skip-download",
            name,
            version,
            path.display()
        );
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_naming_server_handler() {
        assert_eq!(
            interfaces::server_handler("0.1.0"),
            "wasmcp:mcp-v20250618/server-handler@0.1.0"
        );
        assert_eq!(
            interfaces::server_handler("1.0.0"),
            "wasmcp:mcp-v20250618/server-handler@1.0.0"
        );
    }

    #[test]
    fn test_interface_naming_tools() {
        assert_eq!(
            interfaces::tools("0.1.0"),
            "wasmcp:mcp-v20250618/tools@0.1.0"
        );
        assert_eq!(
            interfaces::tools("1.0.0"),
            "wasmcp:mcp-v20250618/tools@1.0.0"
        );
    }

    #[test]
    fn test_package_naming() {
        assert_eq!(
            interfaces::package("http-transport", "0.1.0"),
            "wasmcp:http-transport@0.1.0"
        );
        assert_eq!(
            interfaces::package("method-not-found", "0.1.0"),
            "wasmcp:method-not-found@0.1.0"
        );
        assert_eq!(
            interfaces::package("tools-middleware", "1.0.0"),
            "wasmcp:tools-middleware@1.0.0"
        );
    }

    #[test]
    fn test_get_dependency_path_versioned_nonexistent() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let result = get_dependency_path_versioned("http-transport", "0.1.0", temp_dir.path());

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("http-transport"));
        assert!(err_msg.contains("not found"));
        assert!(err_msg.contains("--skip-download"));
    }

    #[test]
    fn test_get_dependency_path_versioned_exists() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let filename = "wasmcp_http-transport@0.1.0.wasm";
        let file_path = temp_dir.path().join(filename);

        // Create empty file
        std::fs::write(&file_path, b"").unwrap();

        let result = get_dependency_path_versioned("http-transport", "0.1.0", temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), file_path);
    }

    #[test]
    fn test_dependency_filename_format() {
        // Test that the filename format is consistent
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Create a test file
        let filename = "wasmcp_test-component@1.2.3.wasm";
        let file_path = temp_dir.path().join(filename);
        std::fs::write(&file_path, b"").unwrap();

        let result = get_dependency_path_versioned("test-component", "1.2.3", temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().file_name().unwrap(), filename);
    }

    #[test]
    fn test_get_dependency_path_with_resolver() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let resolver = VersionResolver::new().unwrap();

        // Get the expected version from resolver
        let expected_version = resolver.get_version("transport").unwrap();
        let filename = format!("wasmcp_transport@{}.wasm", expected_version);
        let file_path = temp_dir.path().join(filename);

        // Create empty file
        std::fs::write(&file_path, b"").unwrap();

        let result = get_dependency_path("transport", &resolver, temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), file_path);
    }
}
