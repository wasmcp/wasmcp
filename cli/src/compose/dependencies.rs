//! Framework dependency management
//!
//! This module handles downloading and locating wasmcp framework components
//! such as transports, method-not-found handler, and tools-middleware.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::pkg;

/// Type alias for the package client used throughout composition
pub type PackageClient = wasm_pkg_client::caching::CachingClient<wasm_pkg_client::caching::FileCache>;

/// WIT interface constants for MCP protocol
pub mod interfaces {
    /// WASI HTTP incoming-handler interface (HTTP transport export)
    pub const WASI_HTTP_HANDLER: &str = "wasi:http/incoming-handler@0.2.3";

    /// WASI CLI run interface (stdio transport export)
    pub const WASI_CLI_RUN: &str = "wasi:cli/run@0.2.3";

    /// Generate the server-handler interface name with version
    pub fn server_handler(version: &str) -> String {
        format!("wasmcp:mcp/server-handler@{}", version)
    }

    /// Generate the tools-capability interface name with version
    pub fn tools_capability(version: &str) -> String {
        format!("wasmcp:mcp/tools-capability@{}", version)
    }

    /// Generate a versioned package name for wasmcp components
    pub fn package(name: &str, version: &str) -> String {
        format!("wasmcp:{}@{}", name, version)
    }
}

/// Download required framework dependencies (transport, method-not-found, and tools-middleware)
pub async fn download_dependencies(
    transport: &str,
    version: &str,
    deps_dir: &Path,
    client: &PackageClient,
) -> Result<()> {
    let transport_pkg = interfaces::package(&format!("{}-transport", transport), version);
    let method_not_found_pkg = interfaces::package("method-not-found", version);
    let tools_middleware_pkg = interfaces::package("tools-middleware", version);

    let specs = vec![transport_pkg, method_not_found_pkg, tools_middleware_pkg];

    pkg::download_packages(client, &specs, deps_dir).await
}

/// Get the file path for a framework dependency
///
/// Framework dependencies are always stored as `wasmcp_{name}@{version}.wasm`
pub fn get_dependency_path(name: &str, version: &str, deps_dir: &Path) -> Result<PathBuf> {
    let filename = format!("wasmcp_{}@{}.wasm", name, version);
    let path = deps_dir.join(&filename);

    if !path.exists() {
        anyhow::bail!(
            "dependency '{}' not found at '{}', run without --skip-download",
            name,
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
            interfaces::server_handler("0.4.0"),
            "wasmcp:mcp/server-handler@0.4.0"
        );
        assert_eq!(
            interfaces::server_handler("1.0.0"),
            "wasmcp:mcp/server-handler@1.0.0"
        );
    }

    #[test]
    fn test_interface_naming_tools_capability() {
        assert_eq!(
            interfaces::tools_capability("0.4.0"),
            "wasmcp:mcp/tools-capability@0.4.0"
        );
        assert_eq!(
            interfaces::tools_capability("1.0.0"),
            "wasmcp:mcp/tools-capability@1.0.0"
        );
    }

    #[test]
    fn test_package_naming() {
        assert_eq!(
            interfaces::package("http-transport", "0.4.0"),
            "wasmcp:http-transport@0.4.0"
        );
        assert_eq!(
            interfaces::package("method-not-found", "0.4.0"),
            "wasmcp:method-not-found@0.4.0"
        );
        assert_eq!(
            interfaces::package("tools-middleware", "1.0.0"),
            "wasmcp:tools-middleware@1.0.0"
        );
    }

    #[test]
    fn test_wasi_interface_constants() {
        assert_eq!(
            interfaces::WASI_HTTP_HANDLER,
            "wasi:http/incoming-handler@0.2.3"
        );
        assert_eq!(interfaces::WASI_CLI_RUN, "wasi:cli/run@0.2.3");
    }

    #[test]
    fn test_get_dependency_path_nonexistent() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let result = get_dependency_path("http-transport", "0.4.0", temp_dir.path());

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("http-transport"));
        assert!(err_msg.contains("not found"));
        assert!(err_msg.contains("--skip-download"));
    }

    #[test]
    fn test_get_dependency_path_exists() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let filename = "wasmcp_http-transport@0.4.0.wasm";
        let file_path = temp_dir.path().join(filename);

        // Create empty file
        std::fs::write(&file_path, b"").unwrap();

        let result = get_dependency_path("http-transport", "0.4.0", temp_dir.path());
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

        let result = get_dependency_path("test-component", "1.2.3", temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().file_name().unwrap(), filename);
    }
}
