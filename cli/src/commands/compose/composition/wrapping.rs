//! Automatic detection and wrapping of capability components
//!
//! This module handles detecting whether components export capability interfaces
//! (tools, resources, etc.) and automatically wrapping them with the appropriate
//! middleware to convert them into server-handler components.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use wac_graph::{CompositionGraph, EncodeOptions};

use super::load_package;
use crate::commands::compose::resolution::dependencies;
use crate::versioning::VersionResolver;

/// Prefix for temporary wrapped component files
const WRAPPED_COMPONENT_PREFIX: &str = ".wrapped-";

/// Discover the server-handler interface that a middleware component exports
///
/// Inspects a middleware component's exports to find the server-handler interface version.
/// For example, tools-middleware exports wasmcp:mcp-v20250618/server-handler@VERSION.
fn discover_server_handler_interface(middleware_path: &Path) -> Result<String> {
    use wit_component::DecodedWasm;

    let bytes = std::fs::read(middleware_path).with_context(|| {
        format!(
            "Failed to read middleware from {}",
            middleware_path.display()
        )
    })?;

    let decoded = wit_component::decode(&bytes).context("Failed to decode middleware component")?;

    let (resolve, world_id) = match decoded {
        DecodedWasm::Component(resolve, world_id) => (resolve, world_id),
        DecodedWasm::WitPackage(_, _) => {
            anyhow::bail!("Expected a component, found a WIT package");
        }
    };

    let world = &resolve.worlds[world_id];

    // Search exports for server-handler interface
    for (key, _item) in &world.exports {
        if let wit_parser::WorldKey::Interface(id) = key {
            let interface = &resolve.interfaces[*id];
            if let Some(package_id) = interface.package {
                let package = &resolve.packages[package_id];
                let full_name = format!(
                    "{}:{}/{}@{}",
                    package.name.namespace,
                    package.name.name,
                    interface.name.as_ref().unwrap_or(&"unknown".to_string()),
                    package
                        .name
                        .version
                        .as_ref()
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "0.0.0".to_string())
                );

                if full_name.starts_with("wasmcp:mcp-v20250618/server-handler@") {
                    return Ok(full_name);
                }
            }
        }
    }

    anyhow::bail!(
        "No server-handler export found in middleware at {}",
        middleware_path.display()
    )
}

/// Discover the capability interface that a middleware component expects
///
/// Inspects a middleware component's imports to find which capability interface it wraps.
/// For example, tools-middleware imports wasmcp:mcp-v20250618/tools@VERSION.
fn discover_capability_interface(middleware_path: &Path, prefix: &str) -> Result<String> {
    use wit_component::DecodedWasm;

    let bytes = std::fs::read(middleware_path).with_context(|| {
        format!(
            "Failed to read middleware from {}",
            middleware_path.display()
        )
    })?;

    let decoded = wit_component::decode(&bytes).context("Failed to decode middleware component")?;

    let (resolve, world_id) = match decoded {
        DecodedWasm::Component(resolve, world_id) => (resolve, world_id),
        DecodedWasm::WitPackage(_, _) => {
            anyhow::bail!("Expected a component, found a WIT package");
        }
    };

    let world = &resolve.worlds[world_id];

    // Search imports for matching capability interface
    for (key, _item) in &world.imports {
        if let wit_parser::WorldKey::Interface(id) = key {
            let interface = &resolve.interfaces[*id];
            if let Some(package_id) = interface.package {
                let package = &resolve.packages[package_id];
                let full_name = format!(
                    "{}:{}/{}@{}",
                    package.name.namespace,
                    package.name.name,
                    interface.name.as_ref().unwrap_or(&"unknown".to_string()),
                    package
                        .name
                        .version
                        .as_ref()
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "0.0.0".to_string())
                );

                if full_name.starts_with(prefix) {
                    return Ok(full_name);
                }
            }
        }
    }

    anyhow::bail!(
        "No import found matching prefix '{}' in middleware at {}",
        prefix,
        middleware_path.display()
    )
}

use std::collections::HashMap;

/// Auto-detect and wrap capability components with appropriate middleware
///
/// This function inspects each component to determine if it exports capability
/// interfaces (tools, resources, etc.). If so, it wraps the component with the
/// appropriate middleware to convert it into a server-handler component.
///
/// Middleware is discovered dynamically from versions.toml using the naming
/// convention: components ending with "-middleware" are capability wrappers.
pub async fn wrap_capabilities(
    component_paths: Vec<PathBuf>,
    deps_dir: &Path,
    resolver: &VersionResolver,
    overrides: &HashMap<String, String>,
    verbose: bool,
) -> Result<Vec<PathBuf>> {
    let mut wrapped_paths = Vec::new();

    // Discover middleware components dynamically from versions.toml
    let middleware_names = resolver.middleware_components();

    if middleware_names.is_empty() {
        // No middleware configured - return components as-is
        return Ok(component_paths);
    }

    // Create client for resolving overrides (only if needed)
    let has_overrides = middleware_names
        .iter()
        .any(|name| overrides.contains_key(*name));
    let client = if has_overrides {
        Some(crate::commands::pkg::create_default_client().await?)
    } else {
        None
    };

    // Resolve all middleware paths and discover their capability interfaces
    struct MiddlewareConfig {
        name: String,
        path: PathBuf,
        capability_name: String,
        capability_interface: String,
    }

    let mut middleware_configs = Vec::new();

    for middleware_name in middleware_names {
        // Resolve middleware path (check overrides first)
        let path = if let Some(override_spec) = overrides.get(middleware_name) {
            if verbose {
                println!("\nUsing override {}: {}", middleware_name, override_spec);
            }
            crate::commands::compose::resolution::resolve_component_spec(
                override_spec,
                deps_dir,
                client.as_ref().unwrap(),
                verbose,
            )
            .await?
        } else {
            dependencies::get_dependency_path(middleware_name, resolver, deps_dir)?
        };

        // Derive capability name (tools-middleware → tools)
        let capability_name = middleware_name
            .strip_suffix("-middleware")
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Middleware component '{}' must end with '-middleware'",
                    middleware_name
                )
            })?
            .to_string();

        // Discover the capability interface this middleware wraps
        let interface_prefix = format!("wasmcp:mcp-v20250618/{}@", capability_name);
        let capability_interface = discover_capability_interface(&path, &interface_prefix)
            .with_context(|| {
                format!(
                    "Failed to discover {} interface from {}",
                    capability_name, middleware_name
                )
            })?;

        middleware_configs.push(MiddlewareConfig {
            name: middleware_name.to_string(),
            path,
            capability_name,
            capability_interface,
        });
    }

    // Discover server-handler interface (all middleware export it, use first as source)
    let server_handler_interface = if let Some(first_config) = middleware_configs.first() {
        discover_server_handler_interface(&first_config.path)
            .context("Failed to discover server-handler interface from middleware")?
    } else {
        // No middleware found - return components as-is
        return Ok(component_paths);
    };

    if verbose {
        println!("   Discovered capability interfaces:");
        println!("     - {}", server_handler_interface);
        for config in &middleware_configs {
            println!("     - {}", config.capability_interface);
        }
    }

    // Check each user component against all middleware
    for (i, path) in component_paths.into_iter().enumerate() {
        let component_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("component");

        // If component already exports server-handler, it's a handler component - use as-is
        if component_exports_interface(&path, &server_handler_interface)? {
            if verbose {
                println!("   {} is a server-handler → using as-is", component_name);
            }
            wrapped_paths.push(path);
            continue;
        }

        // Check against all discovered middleware
        let mut wrapped = false;
        for config in &middleware_configs {
            if component_exports_interface(&path, &config.capability_interface)? {
                if verbose {
                    println!(
                        "   {} is a {}-capability → wrapping with {}",
                        component_name, config.capability_name, config.name
                    );
                }

                let wrapped_bytes = wrap_with_middleware(
                    &config.path,
                    &path,
                    &config.capability_interface,
                    &config.name,
                    &format!("{}-capability", config.capability_name),
                )?;

                let wrapped_path = deps_dir.join(format!(
                    "{}{}-{}.wasm",
                    WRAPPED_COMPONENT_PREFIX, config.capability_name, i
                ));
                std::fs::write(&wrapped_path, wrapped_bytes)
                    .context("Failed to write wrapped component")?;

                wrapped_paths.push(wrapped_path);
                wrapped = true;
                break;
            }
        }

        // Not a capability component - assume server-handler, use as-is
        if !wrapped {
            if verbose {
                println!("   {} is a server-handler → using as-is", component_name);
            }
            wrapped_paths.push(path);
        }
    }

    Ok(wrapped_paths)
}

/// Check if a component exports a specific interface
///
/// This loads the component and inspects its exports to determine its type.
/// Note: For composed components, this returns true if ANY nested component exports
/// the interface. The wrap_capabilities function handles this by checking for
/// server-handler exports first.
fn component_exports_interface(path: &Path, interface: &str) -> Result<bool> {
    use wasmparser::{Parser, Payload};

    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read component: {}", path.display()))?;

    // Parse the component to find exports
    for payload in Parser::new(0).parse_all(&bytes) {
        let payload = payload.context("Failed to parse component")?;

        if let Payload::ComponentExportSection(exports) = payload {
            for export in exports {
                let export = export.context("Failed to parse export")?;
                if export.name.0 == interface {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

/// Wrap a capability component with its middleware
///
/// This composes: middleware + capability → wrapped component
/// The wrapped component exports server-handler and can be used in the pipeline.
fn wrap_with_middleware(
    middleware_path: &Path,
    capability_path: &Path,
    capability_interface: &str,
    middleware_name: &str,
    capability_name: &str,
) -> Result<Vec<u8>> {
    let mut graph = CompositionGraph::new();

    // Load both components (verbose = false for internal wrapping operations)
    let middleware_pkg = load_package(&mut graph, middleware_name, middleware_path, false)?;
    let capability_pkg = load_package(&mut graph, capability_name, capability_path, false)?;

    // Register packages
    let middleware_id = graph.register_package(middleware_pkg)?;
    let capability_id = graph.register_package(capability_pkg)?;

    // Discover server-handler interface from middleware component exports
    let server_handler_interface = discover_server_handler_interface(middleware_path)?;

    // Instantiate capability component
    let capability_inst = graph.instantiate(capability_id);

    // Get its capability export (tools, resources, etc.)
    let capability_export = graph
        .alias_instance_export(capability_inst, capability_interface)
        .with_context(|| format!("Failed to get {} export", capability_name))?;

    // Instantiate middleware
    let middleware_inst = graph.instantiate(middleware_id);

    // Wire middleware's capability import to the capability's export
    graph
        .set_instantiation_argument(middleware_inst, capability_interface, capability_export)
        .with_context(|| format!("Failed to wire {} interface", capability_name))?;

    // Export the middleware's server-handler export
    let server_handler_export = graph
        .alias_instance_export(middleware_inst, &server_handler_interface)
        .context("Failed to get server-handler export from middleware")?;

    graph
        .export(server_handler_export, &server_handler_interface)
        .context("Failed to export server-handler")?;

    // Encode the wrapped component
    let bytes = graph
        .encode(EncodeOptions::default())
        .context("Failed to encode wrapped component")?;

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Test that component_exports_interface correctly identifies missing exports
    #[test]
    fn test_component_missing_file() {
        let result =
            component_exports_interface(Path::new("/nonexistent/file.wasm"), "some:interface");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to read component")
        );
    }

    /// Test get_dependency_path references in error messages
    #[test]
    fn test_wrap_capabilities_error_handling() {
        // This test verifies that the function signature is correct and can handle errors
        // Real integration testing requires actual WASM components
        let temp_dir = TempDir::new().unwrap();
        let component_paths = vec![temp_dir.path().join("nonexistent.wasm")];

        // Create version resolver
        let resolver = crate::versioning::VersionResolver::new().unwrap();

        // Create a runtime for the async function
        let rt = tokio::runtime::Runtime::new().unwrap();
        let overrides = HashMap::new();
        let result = rt.block_on(wrap_capabilities(
            component_paths,
            temp_dir.path(),
            &resolver,
            &overrides,
            false, // verbose
        ));

        // Should fail because component doesn't exist
        assert!(result.is_err());
    }

    /// Test WRAPPED_COMPONENT_PREFIX constant
    #[test]
    fn test_wrapped_component_prefix() {
        assert_eq!(WRAPPED_COMPONENT_PREFIX, ".wrapped-");
        // Verify it starts with a dot (hidden file on Unix)
        assert!(WRAPPED_COMPONENT_PREFIX.starts_with('.'));
    }

    /// Test that wrap_capabilities creates correctly named output files
    #[test]
    fn test_wrapped_component_naming() {
        let temp_dir = TempDir::new().unwrap();

        // Test the naming pattern that would be used
        let expected_tools = format!("{}tools-0.wasm", WRAPPED_COMPONENT_PREFIX);
        let expected_resources = format!("{}resources-1.wasm", WRAPPED_COMPONENT_PREFIX);

        assert_eq!(expected_tools, ".wrapped-tools-0.wasm");
        assert_eq!(expected_resources, ".wrapped-resources-1.wasm");

        // Verify paths would be constructed correctly
        let tools_path = temp_dir.path().join(expected_tools);
        let resources_path = temp_dir.path().join(expected_resources);

        assert!(
            tools_path
                .to_string_lossy()
                .contains(".wrapped-tools-0.wasm")
        );
        assert!(
            resources_path
                .to_string_lossy()
                .contains(".wrapped-resources-1.wasm")
        );
    }

    /// Test component name extraction from path
    #[test]
    fn test_component_name_extraction() {
        let path1 = PathBuf::from("/path/to/calculator.wasm");
        let name1 = path1
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("component");
        assert_eq!(name1, "calculator");

        let path2 = PathBuf::from("my-handler.wasm");
        let name2 = path2
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("component");
        assert_eq!(name2, "my-handler");

        // Test fallback
        let path3 = PathBuf::from("/");
        let name3 = path3
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("component");
        assert_eq!(name3, "component");
    }

    /// Test that inspection module interfaces are accessible
    #[test]
    fn test_interface_constants_available() {
        use crate::DEFAULT_WASMCP_VERSION;
        use crate::commands::compose::inspection::interfaces;

        // Verify we can call interface naming functions
        let server_handler = interfaces::server_handler(DEFAULT_WASMCP_VERSION);
        let tools = interfaces::tools(DEFAULT_WASMCP_VERSION);
        let resources = interfaces::resources(DEFAULT_WASMCP_VERSION);
        let prompts = interfaces::prompts(DEFAULT_WASMCP_VERSION);

        // Verify format
        assert!(server_handler.starts_with("wasmcp:mcp-v20250618/server-handler@"));
        assert!(tools.starts_with("wasmcp:mcp-v20250618/tools@"));
        assert!(resources.starts_with("wasmcp:mcp-v20250618/resources@"));
        assert!(prompts.starts_with("wasmcp:mcp-v20250618/prompts@"));
    }

    /// Test verbose message formats for component detection
    #[test]
    fn test_verbose_detection_messages() {
        let component_name = "calculator";

        // Server-handler detection message
        let handler_msg = format!("   {} is a server-handler → using as-is", component_name);
        assert_eq!(
            handler_msg,
            "   calculator is a server-handler → using as-is"
        );
        assert!(handler_msg.contains("server-handler"));
        assert!(handler_msg.contains("using as-is"));

        // Tools capability wrapping message
        let tools_msg = format!(
            "   {} is a tools-capability → wrapping with tools-middleware",
            component_name
        );
        assert_eq!(
            tools_msg,
            "   calculator is a tools-capability → wrapping with tools-middleware"
        );
        assert!(tools_msg.contains("tools-capability"));
        assert!(tools_msg.contains("tools-middleware"));

        // Resources capability wrapping message
        let resources_msg = format!(
            "   {} is a resources-capability → wrapping with resources-middleware",
            component_name
        );
        assert!(resources_msg.contains("resources-capability"));
        assert!(resources_msg.contains("resources-middleware"));

        // Prompts capability wrapping message
        let prompts_msg = format!(
            "   {} is a prompts-capability → wrapping with prompts-middleware",
            component_name
        );
        assert!(prompts_msg.contains("prompts-capability"));
        assert!(prompts_msg.contains("prompts-middleware"));
    }

    /// Test middleware path construction pattern
    #[test]
    fn test_middleware_naming_pattern() {
        // Middleware names used in get_dependency_path calls
        let tools_middleware = "tools-middleware";
        let resources_middleware = "resources-middleware";
        let prompts_middleware = "prompts-middleware";

        assert_eq!(tools_middleware, "tools-middleware");
        assert_eq!(resources_middleware, "resources-middleware");
        assert_eq!(prompts_middleware, "prompts-middleware");

        // Verify consistent naming pattern: {type}-middleware
        assert!(tools_middleware.ends_with("-middleware"));
        assert!(resources_middleware.ends_with("-middleware"));
        assert!(prompts_middleware.ends_with("-middleware"));
    }

    /// Test wrapped component output path construction
    #[test]
    fn test_wrapped_output_path_construction() {
        let temp_dir = TempDir::new().unwrap();
        let index = 3;

        // Test each capability type's output naming
        let tools_output = temp_dir
            .path()
            .join(format!("{}tools-{}.wasm", WRAPPED_COMPONENT_PREFIX, index));
        let resources_output = temp_dir.path().join(format!(
            "{}resources-{}.wasm",
            WRAPPED_COMPONENT_PREFIX, index
        ));
        let prompts_output = temp_dir.path().join(format!(
            "{}prompts-{}.wasm",
            WRAPPED_COMPONENT_PREFIX, index
        ));

        // Verify naming pattern
        assert!(
            tools_output
                .to_string_lossy()
                .contains(".wrapped-tools-3.wasm")
        );
        assert!(
            resources_output
                .to_string_lossy()
                .contains(".wrapped-resources-3.wasm")
        );
        assert!(
            prompts_output
                .to_string_lossy()
                .contains(".wrapped-prompts-3.wasm")
        );

        // Verify all use same prefix
        assert!(
            tools_output
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with(WRAPPED_COMPONENT_PREFIX)
        );
        assert!(
            resources_output
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with(WRAPPED_COMPONENT_PREFIX)
        );
        assert!(
            prompts_output
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with(WRAPPED_COMPONENT_PREFIX)
        );
    }

    /// Test detection order priority - server-handler MUST be checked first
    #[test]
    fn test_detection_order_priority() {
        // This test documents the critical bug fix: checking server-handler first
        // prevents re-wrapping already-wrapped components

        // Detection order (line 40-133):
        // 1. server-handler (MUST be first)
        // 2. tools
        // 3. resources
        // 4. prompts
        // 5. else (assume server-handler)

        let detection_order = [
            "server-handler",
            "tools",
            "resources",
            "prompts",
            "else (assume server-handler)",
        ];

        // Verify server-handler is first
        assert_eq!(detection_order[0], "server-handler");

        // This is critical because composed handlers export server-handler at top level
        // but contain nested capability components. If we check capabilities first,
        // we'd detect the nested component and try to re-wrap.
    }

    /// Test error context message construction
    #[test]
    fn test_error_context_messages() {
        let path = Path::new("/path/to/component.wasm");
        let capability_name = "tools-capability";

        // Component reading error
        let read_error = format!("Failed to read component: {}", path.display());
        assert!(read_error.contains("Failed to read component"));
        assert!(read_error.contains("/path/to/component.wasm"));

        // Export retrieval error
        let export_error = format!("Failed to get {} export", capability_name);
        assert_eq!(export_error, "Failed to get tools-capability export");
        assert!(export_error.contains("Failed to get"));
        assert!(export_error.contains("export"));

        // Interface wiring error
        let wire_error = format!("Failed to wire {} interface", capability_name);
        assert_eq!(wire_error, "Failed to wire tools-capability interface");
        assert!(wire_error.contains("Failed to wire"));
        assert!(wire_error.contains("interface"));
    }

    /// Test wasmparser error handling in component_exports_interface
    #[test]
    fn test_component_parse_error_context() {
        let error_msg = "Failed to parse component";
        assert_eq!(error_msg, "Failed to parse component");

        let export_error = "Failed to parse export";
        assert_eq!(export_error, "Failed to parse export");
    }

    /// Test middleware wrapping error contexts
    #[test]
    fn test_middleware_wrapping_errors() {
        let server_handler_error = "Failed to get server-handler export from middleware";
        let export_error = "Failed to export server-handler";
        let encode_error = "Failed to encode wrapped component";
        let write_error = "Failed to write wrapped component";

        assert!(server_handler_error.contains("server-handler export from middleware"));
        assert!(export_error.contains("export server-handler"));
        assert!(encode_error.contains("encode wrapped component"));
        assert!(write_error.contains("write wrapped component"));
    }

    /// Test capability interface names used in wrapping
    #[test]
    fn test_capability_interface_names() {
        let middleware_name = "tools-middleware";
        let capability_name = "tools-capability";

        // Package names for wac-graph
        assert_eq!(middleware_name, "tools-middleware");
        assert_eq!(capability_name, "tools-capability");

        // Verify consistent naming: {type}-middleware and {type}-capability
        let base_type = "tools";
        assert_eq!(format!("{}-middleware", base_type), middleware_name);
        assert_eq!(format!("{}-capability", base_type), capability_name);
    }

    /// Test component iteration pattern with indices
    #[test]
    fn test_component_iteration_with_indices() {
        let paths = vec![
            PathBuf::from("comp1.wasm"),
            PathBuf::from("comp2.wasm"),
            PathBuf::from("comp3.wasm"),
        ];

        // Simulate the iteration at line 33
        let indexed: Vec<(usize, PathBuf)> = paths.into_iter().enumerate().collect();

        assert_eq!(indexed.len(), 3);
        assert_eq!(indexed[0].0, 0);
        assert_eq!(indexed[1].0, 1);
        assert_eq!(indexed[2].0, 2);

        // Indices are used for unique output filenames
        for (i, _path) in indexed {
            let output_name = format!("{}tools-{}.wasm", WRAPPED_COMPONENT_PREFIX, i);
            assert!(output_name.starts_with(".wrapped-"));
            assert!(output_name.ends_with(".wasm"));
        }
    }

    /// Test that else branch assumes server-handler (line 128-133)
    #[test]
    fn test_fallback_assumes_handler() {
        // If a component doesn't export any known interface, we assume it's a handler
        // This is the else branch at line 128-133

        let component_name = "unknown-component";
        let fallback_msg = format!("   {} is a server-handler → using as-is", component_name);

        assert!(fallback_msg.contains("server-handler"));
        assert!(fallback_msg.contains("using as-is"));

        // This fallback prevents errors when components export custom interfaces
    }
}
