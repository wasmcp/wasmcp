use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use wac_graph::{CompositionGraph, EncodeOptions};

use crate::pkg;

/// WIT interface and package name constants
mod interfaces {
    /// Base namespace for all wasmcp MCP interfaces
    pub const NAMESPACE: &str = "wasmcp:mcp";

    /// Package namespace for wasmcp framework components
    pub const PKG_NAMESPACE: &str = "wasmcp";

    /// WASI HTTP handler interface
    pub const WASI_HTTP_HANDLER: &str = "wasi:http/incoming-handler@0.2.3";

    /// WASI CLI run interface
    pub const WASI_CLI_RUN: &str = "wasi:cli/run@0.2.3";

    /// Generate a versioned interface name
    pub fn interface(name: &str, version: &str) -> String {
        format!("{}/{}@{}", NAMESPACE, name, version)
    }

    /// Generate a versioned package name
    pub fn package(name: &str, version: &str) -> String {
        format!("{}:{}@{}", PKG_NAMESPACE, name, version)
    }

    /// Core MCP interfaces
    pub mod core {
        use super::interface;

        pub fn request(version: &str) -> String {
            interface("request", version)
        }

        pub fn incoming_handler(version: &str) -> String {
            interface("incoming-handler", version)
        }

        pub fn error_result(version: &str) -> String {
            interface("error-result", version)
        }

        pub fn initialize_result(version: &str) -> String {
            interface("initialize-result", version)
        }
    }

    /// Tools capability interfaces
    pub mod tools {
        use super::interface;

        pub fn list_result(version: &str) -> String {
            interface("tools-list-result", version)
        }

        pub fn call_content(version: &str) -> String {
            interface("tools-call-content", version)
        }

        pub fn call_structured(version: &str) -> String {
            interface("tools-call-structured", version)
        }
    }

    /// Resources capability interfaces
    pub mod resources {
        use super::interface;

        pub fn list_result(version: &str) -> String {
            interface("resources-list-result", version)
        }

        pub fn read_result(version: &str) -> String {
            interface("resources-read-result", version)
        }

        pub fn templates_list_result(version: &str) -> String {
            interface("resource-templates-list-result", version)
        }
    }

    /// Prompts capability interfaces
    pub mod prompts {
        use super::interface;

        pub fn list_result(version: &str) -> String {
            interface("prompts-list-result", version)
        }

        pub fn get_result(version: &str) -> String {
            interface("prompts-get-result", version)
        }
    }

    /// Completion capability interfaces
    pub mod completion {
        use super::interface;

        pub fn complete_result(version: &str) -> String {
            interface("completion-complete-result", version)
        }
    }
}

/// Handler types supported by MCP
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HandlerType {
    Middleware,
    Tools,
    Resources,
    Prompts,
    Completion,
}

/// Component info: path and detected handler type
#[derive(Debug, Clone)]
struct ComponentInfo {
    path: PathBuf,
    handler_type: HandlerType,
    name: String,
}

/// Override specs for framework components
#[derive(Debug, Clone, Default)]
pub struct ComponentOverrides {
    pub request: Option<String>,
    pub transport: Option<String>,
    pub initialize_handler: Option<String>,
    pub initialize_writer: Option<String>,
    pub error_writer: Option<String>,
    pub tools_writer: Option<String>,
    pub resources_writer: Option<String>,
    pub prompts_writer: Option<String>,
    pub completion_writer: Option<String>,
}

impl std::fmt::Display for HandlerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandlerType::Middleware => write!(f, "middleware"),
            HandlerType::Tools => write!(f, "tools"),
            HandlerType::Resources => write!(f, "resources"),
            HandlerType::Prompts => write!(f, "prompts"),
            HandlerType::Completion => write!(f, "completion"),
        }
    }
}

impl HandlerType {
    /// Get the interface name for WIT (completion is singular, others are plural)
    pub fn interface_name(&self) -> &str {
        match self {
            HandlerType::Middleware => "middleware",
            HandlerType::Tools => "tools",
            HandlerType::Resources => "resources",
            HandlerType::Prompts => "prompts",
            HandlerType::Completion => "completion",
        }
    }
}

/// Resolve a handler spec (path or package spec) to a local path
///
/// - If spec is a file path, validates it exists and returns it
/// - If spec looks like a package spec (namespace:name[@version]), downloads it using pkg client
async fn resolve_handler_spec(
    spec: &str,
    handler_type: HandlerType,
    deps_dir: &Path,
    client: &wasm_pkg_client::caching::CachingClient<wasm_pkg_client::caching::FileCache>,
) -> Result<PathBuf> {
    // Check if spec is a local path (contains / or \, or ends with .wasm)
    if spec.contains('/') || spec.contains('\\') || spec.ends_with(".wasm") {
        let path = PathBuf::from(spec);
        if !path.exists() {
            anyhow::bail!("Handler component not found: {}", spec);
        }
        return Ok(path);
    }

    // Otherwise, treat as package spec and download using pkg client
    println!("      Downloading {} from registry...", spec);

    let path = pkg::resolve_spec(spec, client, deps_dir)
        .await
        .context(format!("Failed to download handler: {}", spec))?;

    // Verify it's the correct handler type
    if let Ok(detected_type) = detect_handler_type_from_component(&path) {
        if detected_type != handler_type {
            anyhow::bail!(
                "Downloaded component {} is type {}, expected {}",
                spec,
                detected_type,
                handler_type
            );
        }
    }

    Ok(path)
}

/// Configuration options for the compose command
pub struct ComposeOptions {
    pub handlers: Vec<(HandlerType, String)>,
    pub transport: String,
    pub output: PathBuf,
    pub version: String,
    pub deps_dir: PathBuf,
    pub skip_download: bool,
    pub force: bool,
    pub overrides: ComponentOverrides,
}

/// Main entry point for the compose command
pub async fn compose(options: ComposeOptions) -> Result<()> {
    let ComposeOptions {
        handlers,
        transport,
        output,
        version,
        deps_dir,
        skip_download,
        force,
        overrides,
    } = options;
    // Check if output already exists
    if output.exists() && !force {
        anyhow::bail!(
            "Output file '{}' already exists. Use --force to overwrite.",
            output.display()
        );
    }

    // Require explicit handler specifications
    if handlers.is_empty() {
        anyhow::bail!(
            "No handlers specified. Use --tools, --resources, --prompts, --completion, or --middleware flags.\n\
             Example: wasmcp compose --tools ./my-tools.wasm --resources ./my-resources.wasm"
        );
    }

    // Create package client for downloading components
    let cache_dir = deps_dir.join(".cache");
    let client = pkg::create_client(&cache_dir)
        .await
        .context("Failed to create package client")?;

    // Resolve handler specs to component paths
    println!("🔍 Resolving handler components...");
    let mut components = Vec::new();

    for (i, (handler_type, spec)) in handlers.iter().enumerate() {
        let path = resolve_handler_spec(spec, *handler_type, &deps_dir, &client).await?;
        let base_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("handler");
        let name = format!("{}-{}", base_name, i);
        println!("   - {}: {}", handler_type, spec);
        components.push(ComponentInfo {
            path,
            handler_type: *handler_type,
            name,
        });
    }

    // Collect unique handler types for dependency download
    let handler_types: Vec<HandlerType> = components
        .iter()
        .map(|c| c.handler_type)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Download dependencies if needed
    if !skip_download {
        println!("\n📦 Downloading dependencies...");
        download_dependencies(&handler_types, &transport, &version, &deps_dir, &client).await?;
    } else {
        println!("\n⏭️  Skipping dependency download");
    }

    // Build composition using wac-graph
    println!("\n🔧 Composing MCP server...");
    let bytes = build_composition(
        &components,
        &transport,
        &version,
        &deps_dir,
        &overrides,
        &client,
    )
    .await?;

    // Write output
    std::fs::write(&output, bytes)
        .context(format!("Failed to write output file: {}", output.display()))?;

    println!("\n✅ Composed: {}", output.display());
    println!("\nTo run the server:");
    println!("  wasmtime serve -Scommon {}", output.display());

    Ok(())
}

/// Detect handler type from a component file by inspecting its WIT
fn detect_handler_type_from_component(component_path: &Path) -> Result<HandlerType> {
    let bytes = std::fs::read(component_path).context(format!(
        "Failed to read component: {}",
        component_path.display()
    ))?;

    // Decode the component using wit-component
    let decoded = wit_component::decode(&bytes).context("Failed to decode component")?;

    // Extract the resolve from the decoded component
    let resolve = match decoded {
        wit_component::DecodedWasm::Component(resolve, _) => resolve,
        wit_component::DecodedWasm::WitPackage(resolve, _) => resolve,
    };

    // Track what we find
    let mut has_incoming_handler_import = false;
    let mut has_incoming_handler_export = false;
    let mut has_writer_import = false;

    // Check all packages in the resolve for handler-specific imports
    for (_, package) in &resolve.packages {
        for (_, interface_id) in &package.interfaces {
            let interface = &resolve.interfaces[*interface_id];
            if let Some(name) = &interface.name {
                if name.contains("tools-list-result")
                    || name.contains("tools-call-content")
                    || name.contains("tools-call-structured")
                {
                    return Ok(HandlerType::Tools);
                } else if name.contains("resources-list-result")
                    || name.contains("resources-read-result")
                    || name.contains("resource-templates-list-result")
                {
                    return Ok(HandlerType::Resources);
                } else if name.contains("prompts-list-result")
                    || name.contains("prompts-get-result")
                {
                    return Ok(HandlerType::Prompts);
                } else if name.contains("completion-complete-result") {
                    return Ok(HandlerType::Completion);
                }
            }
        }

        // Also check worlds
        for (_, world_id) in &package.worlds {
            let world = &resolve.worlds[*world_id];

            // Check imports
            for (key, _) in &world.imports {
                let name = resolve.name_world_key(key);
                if name.contains("tools-list-result")
                    || name.contains("tools-call-content")
                    || name.contains("tools-call-structured")
                {
                    return Ok(HandlerType::Tools);
                } else if name.contains("resources-list-result")
                    || name.contains("resources-read-result")
                    || name.contains("resource-templates-list-result")
                {
                    return Ok(HandlerType::Resources);
                } else if name.contains("prompts-list-result")
                    || name.contains("prompts-get-result")
                {
                    return Ok(HandlerType::Prompts);
                } else if name.contains("completion-complete-result") {
                    return Ok(HandlerType::Completion);
                } else if (name.contains("-result") && name.contains("tools"))
                    || (name.contains("-result") && name.contains("resources"))
                    || (name.contains("-result") && name.contains("prompts"))
                    || (name.contains("-result") && name.contains("completion"))
                {
                    has_writer_import = true;
                } else if name.contains("incoming-handler") {
                    has_incoming_handler_import = true;
                }
            }

            // Check exports
            for (key, _) in &world.exports {
                let name = resolve.name_world_key(key);
                if name.contains("incoming-handler") {
                    has_incoming_handler_export = true;
                }
            }
        }
    }

    // If it imports and exports incoming-handler but has no writer imports, it's middleware
    if has_incoming_handler_import && has_incoming_handler_export && !has_writer_import {
        return Ok(HandlerType::Middleware);
    }

    anyhow::bail!(
        "Could not determine handler type from component: {}",
        component_path.display()
    )
}

/// Download dependencies using pkg client
async fn download_dependencies(
    handler_types: &[HandlerType],
    transport: &str,
    version: &str,
    deps_dir: &Path,
    client: &wasm_pkg_client::caching::CachingClient<wasm_pkg_client::caching::FileCache>,
) -> Result<()> {
    // Base dependencies needed by all handlers
    let mut all_deps = vec![
        interfaces::package("request", version),
        interfaces::package("initialize-writer", version),
        interfaces::package("initialize-handler", version),
        interfaces::package(&format!("{}-transport", transport), version),
        interfaces::package("error-writer", version),
    ];

    // Add writer dependency for each handler type (skip middleware)
    for handler_type in handler_types {
        if *handler_type != HandlerType::Middleware {
            let writer_dep = interfaces::package(
                &format!("{}-writer", handler_type.interface_name()),
                version,
            );
            all_deps.push(writer_dep);
        }
    }

    // Download all dependencies in parallel
    pkg::download_packages(client, &all_deps, deps_dir).await
}

/// Helper to resolve a component path (either from override or deps/)
async fn resolve_component_path(
    name: &str,
    override_spec: &Option<String>,
    version: &str,
    deps_dir: &Path,
    client: &wasm_pkg_client::caching::CachingClient<wasm_pkg_client::caching::FileCache>,
) -> Result<PathBuf> {
    if let Some(spec) = override_spec {
        // Override provided - check if it's a local path
        let path = PathBuf::from(spec);
        if path.exists() {
            return Ok(path);
        }

        // Check if it looks like a package spec (contains :)
        if spec.contains(':') {
            // Download as package spec
            println!("   Downloading override {}...", spec);
            let downloaded_path = pkg::resolve_spec(spec, client, deps_dir)
                .await
                .with_context(|| format!("Failed to download override: {}", spec))?;
            return Ok(downloaded_path);
        }

        // Assume it's in deps/ with transformed filename
        let filename = spec.replace([':', '/'], "_");
        let override_path = deps_dir.join(format!("{}.wasm", filename));
        if override_path.exists() {
            Ok(override_path)
        } else {
            anyhow::bail!(
                "Override component not found: {} (tried {} and {})",
                spec,
                path.display(),
                override_path.display()
            )
        }
    } else {
        // Use default from deps/
        let filename = format!("wasmcp_{}@{}.wasm", name, version);
        let path = deps_dir.join(&filename);
        if !path.exists() {
            anyhow::bail!("Dependency not found: {}", path.display());
        }
        Ok(path)
    }
}

/// Build the composition using wac-graph programmatically
async fn build_composition(
    components: &[ComponentInfo],
    transport: &str,
    version: &str,
    deps_dir: &Path,
    overrides: &ComponentOverrides,
    client: &wasm_pkg_client::caching::CachingClient<wasm_pkg_client::caching::FileCache>,
) -> Result<Vec<u8>> {
    let mut graph = CompositionGraph::new();

    // Helper to load a package from resolved path
    let load_package = |graph: &mut CompositionGraph,
                        name: &str,
                        path: &Path|
     -> Result<wac_graph::types::Package> {
        wac_graph::types::Package::from_file(
            &format!("wasmcp:{}", name),
            None,
            path,
            graph.types_mut(),
        )
        .context(format!("Failed to load package: {}", name))
    };

    println!("   Loading packages...");

    // Load base packages with override support
    let request_path =
        resolve_component_path("request", &overrides.request, version, deps_dir, client).await?;
    let request_package = load_package(&mut graph, "request", &request_path)?;

    let init_writer_path = resolve_component_path(
        "initialize-writer",
        &overrides.initialize_writer,
        version,
        deps_dir,
        client,
    )
    .await?;
    let init_writer_package = load_package(&mut graph, "initialize-writer", &init_writer_path)?;

    let init_handler_path = resolve_component_path(
        "initialize-handler",
        &overrides.initialize_handler,
        version,
        deps_dir,
        client,
    )
    .await?;
    let init_handler_package = load_package(&mut graph, "initialize-handler", &init_handler_path)?;

    let transport_name = format!("{}-transport", transport);
    let transport_path = resolve_component_path(
        &transport_name,
        &overrides.transport,
        version,
        deps_dir,
        client,
    )
    .await?;
    let transport_package = load_package(&mut graph, &transport_name, &transport_path)?;

    let error_writer_path = resolve_component_path(
        "error-writer",
        &overrides.error_writer,
        version,
        deps_dir,
        client,
    )
    .await?;
    let error_writer_package = load_package(&mut graph, "error-writer", &error_writer_path)?;

    // Load writer packages for each unique handler type (skip middleware)
    let mut writer_packages = std::collections::HashMap::new();
    for component in components {
        let handler_type = component.handler_type;
        // Middleware doesn't have a writer
        if handler_type != HandlerType::Middleware
            && !writer_packages.contains_key(&(handler_type as u8))
        {
            let writer_name = format!("{}-writer", handler_type.interface_name());

            // Select appropriate override based on handler type
            let override_spec = match handler_type {
                HandlerType::Tools => &overrides.tools_writer,
                HandlerType::Resources => &overrides.resources_writer,
                HandlerType::Prompts => &overrides.prompts_writer,
                HandlerType::Completion => &overrides.completion_writer,
                HandlerType::Middleware => &None, // Never reached due to check above
            };

            let writer_path =
                resolve_component_path(&writer_name, override_spec, version, deps_dir, client)
                    .await?;
            let writer_package = load_package(&mut graph, &writer_name, &writer_path)?;
            writer_packages.insert(handler_type as u8, (handler_type, writer_package));
        }
    }

    // Load user components
    let mut user_packages = Vec::new();
    for component in components {
        let user_package = wac_graph::types::Package::from_file(
            &format!("wasmcp:{}", component.name),
            None,
            &component.path,
            graph.types_mut(),
        )?;
        user_packages.push(user_package);
    }

    println!("   Registering packages...");

    // Register base packages
    let request_pkg = graph.register_package(request_package)?;
    let init_writer_pkg = graph.register_package(init_writer_package)?;
    let init_handler_pkg = graph.register_package(init_handler_package)?;
    let transport_pkg = graph.register_package(transport_package)?;
    let error_writer_pkg = graph.register_package(error_writer_package)?;

    // Register writer packages
    let mut registered_writers = std::collections::HashMap::new();
    for (key, (handler_type, package)) in writer_packages {
        let pkg_id = graph.register_package(package)?;
        registered_writers.insert(key, (handler_type, pkg_id));
    }

    // Register user packages
    let mut registered_users = Vec::new();
    for package in user_packages {
        let pkg_id = graph.register_package(package)?;
        registered_users.push(pkg_id);
    }

    println!("   Building composition graph...");

    // Instantiate shared request component
    let request_inst = graph.instantiate(request_pkg);
    let request_export =
        graph.alias_instance_export(request_inst, &interfaces::core::request(version))?;

    // Instantiate initialize-writer
    let init_writer_inst = graph.instantiate(init_writer_pkg);
    let init_result_export = graph.alias_instance_export(
        init_writer_inst,
        &interfaces::core::initialize_result(version),
    )?;

    // Instantiate all handler writers and alias their result interface exports
    let mut writer_instances = std::collections::HashMap::new();
    for (key, (handler_type, pkg_id)) in registered_writers {
        let inst = graph.instantiate(pkg_id);
        writer_instances.insert(key, (handler_type, inst));
    }

    // Instantiate error-writer
    let error_writer_inst = graph.instantiate(error_writer_pkg);
    let error_result_export =
        graph.alias_instance_export(error_writer_inst, &interfaces::core::error_result(version))?;

    // Instantiate initialize-handler (terminal handler)
    let init_handler_inst = graph.instantiate(init_handler_pkg);
    graph.set_instantiation_argument(
        init_handler_inst,
        &interfaces::core::request(version),
        request_export,
    )?;
    graph.set_instantiation_argument(
        init_handler_inst,
        &interfaces::core::initialize_result(version),
        init_result_export,
    )?;

    let mut next_handler_export = graph.alias_instance_export(
        init_handler_inst,
        &interfaces::core::incoming_handler(version),
    )?;

    // Chain user handlers in reverse order (so they get called in forward order)
    // Chain: http → handler[0] → handler[1] → ... → init
    for (component, pkg_id) in components.iter().zip(registered_users.iter()).rev() {
        println!("   Wiring handler: {}", component.name);

        let user_inst = graph.instantiate(*pkg_id);

        // Wire shared request
        graph.set_instantiation_argument(
            user_inst,
            &interfaces::core::request(version),
            request_export,
        )?;

        // Wire result interface imports (skip for middleware)
        if component.handler_type != HandlerType::Middleware {
            // All non-middleware handlers import error-result
            graph.set_instantiation_argument(
                user_inst,
                &interfaces::core::error_result(version),
                error_result_export,
            )?;

            // Wire handler-type-specific result interfaces
            let (_, writer_inst) = writer_instances
                .get(&(component.handler_type as u8))
                .ok_or_else(|| anyhow::anyhow!("Writer not found for handler type"))?;

            match component.handler_type {
                HandlerType::Tools => {
                    let tools_list = graph.alias_instance_export(
                        *writer_inst,
                        &interfaces::tools::list_result(version),
                    )?;
                    let tools_call_content = graph.alias_instance_export(
                        *writer_inst,
                        &interfaces::tools::call_content(version),
                    )?;
                    graph.set_instantiation_argument(
                        user_inst,
                        &interfaces::tools::list_result(version),
                        tools_list,
                    )?;
                    graph.set_instantiation_argument(
                        user_inst,
                        &interfaces::tools::call_content(version),
                        tools_call_content,
                    )?;

                    // tools-call-structured is optional (cargo-component may optimize it away if unused)
                    if let Ok(tools_call_structured) = graph.alias_instance_export(
                        *writer_inst,
                        &interfaces::tools::call_structured(version),
                    ) {
                        let _ = graph.set_instantiation_argument(
                            user_inst,
                            &interfaces::tools::call_structured(version),
                            tools_call_structured,
                        );
                    }
                }
                HandlerType::Resources => {
                    let resources_list = graph.alias_instance_export(
                        *writer_inst,
                        &interfaces::resources::list_result(version),
                    )?;
                    let resources_read = graph.alias_instance_export(
                        *writer_inst,
                        &interfaces::resources::read_result(version),
                    )?;
                    let resource_templates = graph.alias_instance_export(
                        *writer_inst,
                        &interfaces::resources::templates_list_result(version),
                    )?;
                    graph.set_instantiation_argument(
                        user_inst,
                        &interfaces::resources::list_result(version),
                        resources_list,
                    )?;
                    graph.set_instantiation_argument(
                        user_inst,
                        &interfaces::resources::read_result(version),
                        resources_read,
                    )?;
                    graph.set_instantiation_argument(
                        user_inst,
                        &interfaces::resources::templates_list_result(version),
                        resource_templates,
                    )?;
                }
                HandlerType::Prompts => {
                    let prompts_list = graph.alias_instance_export(
                        *writer_inst,
                        &interfaces::prompts::list_result(version),
                    )?;
                    let prompts_get = graph.alias_instance_export(
                        *writer_inst,
                        &interfaces::prompts::get_result(version),
                    )?;
                    graph.set_instantiation_argument(
                        user_inst,
                        &interfaces::prompts::list_result(version),
                        prompts_list,
                    )?;
                    graph.set_instantiation_argument(
                        user_inst,
                        &interfaces::prompts::get_result(version),
                        prompts_get,
                    )?;
                }
                HandlerType::Completion => {
                    let completion_result = graph.alias_instance_export(
                        *writer_inst,
                        &interfaces::completion::complete_result(version),
                    )?;
                    graph.set_instantiation_argument(
                        user_inst,
                        &interfaces::completion::complete_result(version),
                        completion_result,
                    )?;
                }
                HandlerType::Middleware => unreachable!(), // Already filtered out above
            }
        }

        // Wire next handler in chain
        graph.set_instantiation_argument(
            user_inst,
            &interfaces::core::incoming_handler(version),
            next_handler_export,
        )?;

        // This handler's export becomes the next handler's input
        next_handler_export =
            graph.alias_instance_export(user_inst, &interfaces::core::incoming_handler(version))?;
    }

    // Instantiate transport at the front of the chain
    let transport_inst = graph.instantiate(transport_pkg);
    graph.set_instantiation_argument(
        transport_inst,
        &interfaces::core::request(version),
        request_export,
    )?;
    graph.set_instantiation_argument(
        transport_inst,
        &interfaces::core::incoming_handler(version),
        next_handler_export,
    )?;

    // Export the appropriate transport interface based on transport type
    match transport {
        "http" => {
            let http_handler =
                graph.alias_instance_export(transport_inst, interfaces::WASI_HTTP_HANDLER)?;
            graph.export(http_handler, interfaces::WASI_HTTP_HANDLER)?;
        }
        "stdio" => {
            let cli_run = graph.alias_instance_export(transport_inst, interfaces::WASI_CLI_RUN)?;
            graph.export(cli_run, interfaces::WASI_CLI_RUN)?;
        }
        _ => anyhow::bail!("Unsupported transport type: {}", transport),
    }

    println!("   Encoding component...");
    let bytes = graph.encode(EncodeOptions::default())?;

    Ok(bytes)
}
