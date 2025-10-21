//! Framework dependency management
//!
//! This module handles downloading and locating wasmcp framework components
//! such as transports, method-not-found handler, and tools-middleware.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::commands::pkg;

/// Type alias for the package client used throughout composition
pub type PackageClient =
    wasm_pkg_client::caching::CachingClient<wasm_pkg_client::caching::FileCache>;

/// WIT interface constants for MCP protocol
pub mod interfaces {
    /// WASI HTTP incoming-handler interface (HTTP transport export)
    pub const WASI_HTTP_HANDLER: &str = "wasi:http/incoming-handler@0.2.3";

    /// WASI CLI run interface (stdio transport export)
    pub const WASI_CLI_RUN: &str = "wasi:cli/run@0.2.3";

    /// Generate the server handler interface name with version
    pub fn server_handler(version: &str) -> String {
        format!("wasmcp:server/handler@{}", version)
    }

    /// Generate the tools capability interface name with version
    pub fn tools(version: &str) -> String {
        format!("wasmcp:protocol/tools@{}", version)
    }

    /// Generate the resources capability interface name with version
    pub fn resources(version: &str) -> String {
        format!("wasmcp:protocol/resources@{}", version)
    }

    /// Generate the prompts capability interface name with version
    pub fn prompts(version: &str) -> String {
        format!("wasmcp:protocol/prompts@{}", version)
    }

    /// Generate a versioned package name for wasmcp components
    pub fn package(name: &str, version: &str) -> String {
        format!("wasmcp:{}@{}", name, version)
    }
}

/// Download required framework dependencies (transport, method-not-found, and all middleware)
pub async fn download_dependencies(
    transport: &str,
    version: &str,
    deps_dir: &Path,
    client: &PackageClient,
) -> Result<()> {
    let transport_pkg = interfaces::package(&format!("{}-transport", transport), version);
    let method_not_found_pkg = interfaces::package("method-not-found", version);
    let tools_middleware_pkg = interfaces::package("tools-middleware", version);
    let resources_middleware_pkg = interfaces::package("resources-middleware", version);
    let prompts_middleware_pkg = interfaces::package("prompts-middleware", version);

    let specs = vec![
        transport_pkg,
        method_not_found_pkg,
        tools_middleware_pkg,
        resources_middleware_pkg,
        prompts_middleware_pkg,
    ];

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
            interfaces::server_handler("0.1.0-beta.2"),
            "wasmcp:server/handler@0.1.0-beta.2"
        );
        assert_eq!(
            interfaces::server_handler("1.0.0"),
            "wasmcp:server/handler@1.0.0"
        );
    }

    #[test]
    fn test_interface_naming_tools() {
        assert_eq!(
            interfaces::tools("0.1.0-beta.2"),
            "wasmcp:protocol/tools@0.1.0-beta.2"
        );
        assert_eq!(interfaces::tools("1.0.0"), "wasmcp:protocol/tools@1.0.0");
    }

    #[test]
    fn test_package_naming() {
        assert_eq!(
            interfaces::package("http-transport", "0.1.0-beta.2"),
            "wasmcp:http-transport@0.1.0-beta.2"
        );
        assert_eq!(
            interfaces::package("method-not-found", "0.1.0-beta.2"),
            "wasmcp:method-not-found@0.1.0-beta.2"
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
        let result = get_dependency_path("http-transport", "0.1.0-beta.2", temp_dir.path());

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
        let filename = "wasmcp_http-transport@0.1.0-beta.2.wasm";
        let file_path = temp_dir.path().join(filename);

        // Create empty file
        std::fs::write(&file_path, b"").unwrap();

        let result = get_dependency_path("http-transport", "0.1.0-beta.2", temp_dir.path());
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
