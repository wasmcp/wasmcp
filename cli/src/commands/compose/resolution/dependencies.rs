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

/// Download required framework dependencies (transport, server-io, session-store variants, method-not-found, and all middleware)
pub async fn download_dependencies(
    resolver: &VersionResolver,
    deps_dir: &Path,
    client: &PackageClient,
) -> Result<()> {
    // Get component-specific versions from the resolver
    let transport_version = resolver.get_version("transport")?;
    let server_io_version = resolver.get_version("server-io")?;
    let session_store_version = resolver.get_version("session-store")?;
    let method_not_found_version = resolver.get_version("method-not-found")?;
    let tools_middleware_version = resolver.get_version("tools-middleware")?;
    let resources_middleware_version = resolver.get_version("resources-middleware")?;
    let prompts_middleware_version = resolver.get_version("prompts-middleware")?;

    // Build package specs with component-specific versions
    // Download both session-store variants (d2 for Spin, standard for wasmcloud/wasmtime)
    let specs = vec![
        interfaces::package("transport", &transport_version),
        interfaces::package("server-io", &server_io_version),
        interfaces::package("session-store", &session_store_version),
        interfaces::package("session-store-d2", &session_store_version),
        interfaces::package("method-not-found", &method_not_found_version),
        interfaces::package("tools-middleware", &tools_middleware_version),
        interfaces::package("resources-middleware", &resources_middleware_version),
        interfaces::package("prompts-middleware", &prompts_middleware_version),
    ];

    pkg::download_packages(client, &specs, deps_dir).await
}

/// Get the file path for a framework dependency
///
/// Framework dependencies are always stored as `wasmcp_{name}@{version}.wasm`
/// Special case: session-store-d2 uses the version from session-store entry
pub fn get_dependency_path(
    name: &str,
    resolver: &VersionResolver,
    deps_dir: &Path,
) -> Result<PathBuf> {
    // session-store-d2 uses the same version as session-store
    let version_key = if name == "session-store-d2" {
        "session-store"
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
