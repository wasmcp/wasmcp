//! Framework dependency management
//!
//! This module handles downloading and locating wasmcp framework components
//! such as transports, method-not-found handler, and tools-middleware.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::commands::pkg;
use crate::versioning::VersionResolver;

use crate::commands::compose::inspection::interfaces;
use crate::commands::compose::inspection::interfaces::ComponentType;

/// Type alias for the package client used throughout composition
pub type PackageClient =
    wasm_pkg_client::caching::CachingClient<wasm_pkg_client::caching::FileCache>;

use std::collections::{HashMap, HashSet};

/// Configuration for which dependencies to skip downloading
pub struct DownloadConfig<'a> {
    pub overrides: &'a HashMap<String, String>,
    pub resolver: &'a VersionResolver,
}

/// Map a WIT interface import to a framework component name
///
/// Returns None if the import is not a wasmcp framework component
fn map_interface_to_component(interface: &str) -> Option<&'static str> {
    // Interface format: "namespace:package/interface@version"
    // Examples:
    // - "wasmcp:mcp-v20250618/server-io@0.1.7" -> "server-io"
    // - "wasmcp:keyvalue/store@0.1.0" -> "kv-store"
    // - "wasmcp:mcp-v20250618/tools@0.1.7" -> "tools-middleware"

    if !interface.starts_with("wasmcp:") {
        return None;
    }

    // Extract the interface name (between / and @)
    let parts: Vec<&str> = interface.split('/').collect();
    if parts.len() != 2 {
        return None;
    }

    let interface_name = parts[1].split('@').next()?;

    // Map interface names to component names
    match interface_name {
        "server-transport" => Some(ComponentType::HttpTransport.name()),
        "server-io" => Some(ComponentType::ServerIo.name()),
        "server-handler" => Some(ComponentType::MethodNotFound.name()),
        "server-auth" => Some(ComponentType::Authorization.name()),
        "tools" => Some(ComponentType::ToolsMiddleware.name()),
        "resources" => Some(ComponentType::ResourcesMiddleware.name()),
        "prompts" => Some(ComponentType::PromptsMiddleware.name()),
        "store" if interface.contains("keyvalue") => Some(ComponentType::KvStore.name()),
        "sessions" | "session-manager" => Some(ComponentType::SessionStore.name()),
        _ => None,
    }
}

/// Discover required dependencies by inspecting component imports
///
/// Analyzes the WIT imports of all user-provided components to determine
/// which framework dependencies are actually needed.
pub fn discover_required_dependencies(
    component_paths: &[PathBuf],
    overrides: &HashMap<String, String>,
) -> Result<HashSet<String>> {
    let mut required = HashSet::new();

    for component_path in component_paths {
        let imports = crate::commands::compose::inspection::check_component_imports(component_path)?;

        for import in imports {
            if let Some(component) = map_interface_to_component(&import) {
                // Only add if not overridden
                if !overrides.contains_key(component) {
                    required.insert(component.to_string());
                }
            }
        }
    }

    Ok(required)
}

impl<'a> DownloadConfig<'a> {
    /// Create config from overrides HashMap
    pub fn new(overrides: &'a HashMap<String, String>, resolver: &'a VersionResolver) -> Self {
        Self {
            overrides,
            resolver,
        }
    }
}

/// Download required framework dependencies based on component imports
///
/// Inspects the provided component paths to discover which framework dependencies
/// are actually imported, then downloads only those dependencies (excluding overrides).
pub async fn download_dependencies(
    component_paths: &[PathBuf],
    config: &DownloadConfig<'_>,
    deps_dir: &Path,
    client: &PackageClient,
) -> Result<()> {
    // Discover what's actually needed by inspecting component imports
    let required = discover_required_dependencies(component_paths, config.overrides)?;

    if required.is_empty() {
        return Ok(());
    }

    let mut specs = Vec::new();

    for component in required {
        let version = config.resolver.get_version(&component)?;
        specs.push(interfaces::package(&component, &version));

        // Special case: kv-store has a draft2 variant
        if component == ComponentType::KvStore.name() {
            specs.push(interfaces::package("kv-store-d2", &version));
        }
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
        ComponentType::SessionStore.name()
    } else if name == "kv-store-d2" {
        ComponentType::KvStore.name()
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

    #[test]
    fn test_map_interface_to_component() {
        // Test mcp-v20250618 interfaces
        assert_eq!(
            map_interface_to_component("wasmcp:mcp-v20250618/server-transport@0.1.7"),
            Some("transport")
        );
        assert_eq!(
            map_interface_to_component("wasmcp:mcp-v20250618/server-io@0.1.7"),
            Some("server-io")
        );
        assert_eq!(
            map_interface_to_component("wasmcp:mcp-v20250618/server-handler@0.1.7"),
            Some("method-not-found")
        );
        assert_eq!(
            map_interface_to_component("wasmcp:mcp-v20250618/server-auth@0.1.7"),
            Some("authorization")
        );
        assert_eq!(
            map_interface_to_component("wasmcp:mcp-v20250618/tools@0.1.7"),
            Some("tools-middleware")
        );
        assert_eq!(
            map_interface_to_component("wasmcp:mcp-v20250618/resources@0.1.7"),
            Some("resources-middleware")
        );
        assert_eq!(
            map_interface_to_component("wasmcp:mcp-v20250618/prompts@0.1.7"),
            Some("prompts-middleware")
        );

        // Test keyvalue interface
        assert_eq!(
            map_interface_to_component("wasmcp:keyvalue/store@0.1.0"),
            Some("kv-store")
        );

        // Test session interfaces
        assert_eq!(
            map_interface_to_component("wasmcp:mcp-v20250618/sessions@0.1.7"),
            Some("session-store")
        );
        assert_eq!(
            map_interface_to_component("wasmcp:mcp-v20250618/session-manager@0.1.7"),
            Some("session-store")
        );

        // Test non-wasmcp interfaces
        assert_eq!(
            map_interface_to_component("wasi:http/outgoing-handler@0.2.8"),
            None
        );

        // Test unknown wasmcp interface
        assert_eq!(
            map_interface_to_component("wasmcp:unknown/interface@1.0.0"),
            None
        );
    }
}
