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

/// Configuration for which dependencies to download
pub struct DownloadConfig<'a> {
    pub overrides: &'a HashMap<String, String>,
    pub resolver: &'a VersionResolver,
    pub required_middleware: &'a [String],
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
/// Analyzes the WIT imports of user-provided components to determine
/// which framework dependencies are actually needed.
pub fn discover_required_dependencies(
    component_paths: &[PathBuf],
) -> Result<HashSet<String>> {
    let mut required = HashSet::new();

    for component_path in component_paths {
        let imports =
            crate::commands::compose::inspection::check_component_imports(component_path)?;

        for import in imports {
            if let Some(component) = map_interface_to_component(&import) {
                // Always add to required set - overrides just change where we get the file
                required.insert(component.to_string());
            }
        }
    }

    Ok(required)
}

/// Discover transitive dependencies from already-resolved framework components
///
/// Inspects framework component files to find their dependencies (e.g., authorization imports kv-store).
/// Downloads new dependencies as they're discovered so they can be inspected.
async fn discover_transitive_dependencies(
    initial_deps: &HashSet<String>,
    resolver: &VersionResolver,
    deps_dir: &Path,
    overrides: &HashMap<String, String>,
    client: &PackageClient,
) -> Result<HashSet<String>> {
    let mut all_deps = initial_deps.clone();
    let mut to_inspect: Vec<String> = initial_deps.iter().cloned().collect();
    let mut inspected = HashSet::new();

    while let Some(dep_name) = to_inspect.pop() {
        // Skip if already inspected
        if !inspected.insert(dep_name.clone()) {
            continue;
        }

        // Get the path to this dependency
        let dep_path = get_dependency_path(&dep_name, resolver, deps_dir)?;

        // Inspect its imports
        let imports = crate::commands::compose::inspection::check_component_imports(&dep_path)?;

        for import in imports {
            if let Some(component_name) = map_interface_to_component(&import) {
                // Always add to all_deps (overrides just change where we get the file)
                if all_deps.insert(component_name.to_string()) {
                    // New dependency found
                    if !overrides.contains_key(component_name) {
                        // Not overridden - download it so we can inspect it
                        let version = resolver.get_version(component_name)?;
                        let mut specs = vec![interfaces::package(component_name, &version)];

                        // Special case: kv-store has a draft2 variant
                        if component_name == ComponentType::KvStore.name() {
                            specs.push(interfaces::package("kv-store-d2", &version));
                        }

                        pkg::download_packages(client, &specs, deps_dir).await?;

                        // Add to inspection queue (we need to inspect it for further deps)
                        to_inspect.push(component_name.to_string());
                    }
                    // If overridden, we don't download but still add to all_deps
                    // We don't inspect overridden components for transitive deps since we can't reliably get their path yet
                }
            }
        }
    }

    Ok(all_deps)
}

impl<'a> DownloadConfig<'a> {
    /// Create config with required middleware list
    pub fn new(
        overrides: &'a HashMap<String, String>,
        resolver: &'a VersionResolver,
        required_middleware: &'a [String],
    ) -> Self {
        Self {
            overrides,
            resolver,
            required_middleware,
        }
    }
}

/// Download required framework dependencies (with transitive dependency resolution)
///
/// Downloads only what's needed:
/// 1. Structural components (transport, method-not-found) - always needed
/// 2. Middleware components from the required list - discovered by inspecting exports
/// 3. Service components from discovered dependencies - discovered by inspecting imports
/// 4. Transitive dependencies - inspects downloaded services to find their dependencies
///
/// Returns the complete set of all downloaded dependencies (including transitive ones)
pub async fn download_dependencies(
    component_paths: &[PathBuf],
    config: &DownloadConfig<'_>,
    deps_dir: &Path,
    client: &PackageClient,
) -> Result<HashSet<String>> {
    // Discover what's actually needed by inspecting component imports
    let mut required = discover_required_dependencies(component_paths)?;

    // ALWAYS include structural components
    // Transport is always at the front of the pipeline
    required.insert(ComponentType::HttpTransport.name().to_string());
    // Method-not-found is always the terminal handler
    required.insert(ComponentType::MethodNotFound.name().to_string());
    // Session-store is always needed (transport depends on it)
    required.insert(ComponentType::SessionStore.name().to_string());

    // Include only the middleware that was discovered as needed
    // We already inspected component exports to determine which middleware is required
    for middleware_name in config.required_middleware {
        if !config.overrides.contains_key(middleware_name.as_str()) {
            required.insert(middleware_name.clone());
        }
    }

    if required.is_empty() {
        return Ok(HashSet::new());
    }

    // Download initial set
    let mut specs = Vec::new();
    for component in &required {
        let version = config.resolver.get_version(component)?;
        specs.push(interfaces::package(component, &version));

        // Special case: kv-store has a draft2 variant
        if component == ComponentType::KvStore.name() {
            specs.push(interfaces::package("kv-store-d2", &version));
        }
    }
    pkg::download_packages(client, &specs, deps_dir).await?;

    // Now discover transitive dependencies by inspecting what we just downloaded
    // This function will download new dependencies as it discovers them
    let all_deps = discover_transitive_dependencies(
        &required,
        config.resolver,
        deps_dir,
        config.overrides,
        client,
    )
    .await?;

    Ok(all_deps)
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
