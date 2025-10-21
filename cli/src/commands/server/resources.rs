use rmcp::model::*;
use rmcp::ErrorData as McpError;
use std::path::PathBuf;
use crate::config::WasmcpConfig;

pub struct WasmcpResources;

impl WasmcpResources {
    pub fn list_all(project_root: &PathBuf) -> Result<ListResourcesResult, McpError> {
        let mut resources = vec![
            // Documentation resources
            RawResource::new(
                "wasmcp://docs/readme",
                "wasmcp Project README",
            )
            .no_annotation(),

            RawResource {
                uri: "wasmcp://docs/cli-reference".into(),
                name: "CLI Command Reference".into(),
                mime_type: Some("text/markdown".into()),
                title: None,
                description: None,
                size: None,
                icons: None,
            }
            .no_annotation(),

            RawResource {
                uri: "wasmcp://docs/wit-protocol".into(),
                name: "WIT Protocol Specification".into(),
                mime_type: Some("text/plain".into()),
                title: None,
                description: None,
                size: None,
                icons: None,
            }
            .no_annotation(),

            // Registry resources
            RawResource {
                uri: "wasmcp://registry/components".into(),
                name: "Registry Component Aliases".into(),
                mime_type: Some("application/json".into()),
                title: None,
                description: None,
                size: None,
                icons: None,
            }
            .no_annotation(),

            RawResource {
                uri: "wasmcp://registry/profiles".into(),
                name: "Registry Composition Profiles".into(),
                mime_type: Some("application/json".into()),
                title: None,
                description: None,
                size: None,
                icons: None,
            }
            .no_annotation(),

            RawResource {
                uri: "wasmcp://registry/config".into(),
                name: "Full Registry Configuration".into(),
                mime_type: Some("application/toml".into()),
                title: None,
                description: None,
                size: None,
                icons: None,
            }
            .no_annotation(),
        ];

        // Add example resources if examples directory exists
        if project_root.join("examples").exists() {
            resources.push(
                RawResource {
                    uri: "wasmcp://docs/examples/calculator".into(),
                    name: "Calculator Example (Rust)".into(),
                    mime_type: Some("text/markdown".into()),
                    title: None,
                    description: None,
                    size: None,
                    icons: None,
                }
                .no_annotation()
            );
        }

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
        })
    }

    pub fn read(
        uri: &str,
        project_root: &PathBuf,
        config: &WasmcpConfig,
    ) -> Result<ReadResourceResult, McpError> {
        let contents = match uri {
            // Documentation
            "wasmcp://docs/readme" => {
                Self::read_readme(project_root)?
            }
            "wasmcp://docs/cli-reference" => {
                Self::generate_cli_reference()?
            }
            "wasmcp://docs/wit-protocol" => {
                Self::read_wit_protocol(project_root)?
            }
            "wasmcp://docs/examples/calculator" => {
                Self::read_example_docs(project_root, "calculator-rs")?
            }

            // Registry
            "wasmcp://registry/components" => {
                Self::format_registry_components(config)?
            }
            "wasmcp://registry/profiles" => {
                Self::format_registry_profiles(config)?
            }
            "wasmcp://registry/config" => {
                Self::read_registry_config(config)?
            }

            _ => {
                return Err(McpError::resource_not_found(
                    format!("Unknown resource URI: {}", uri),
                    None
                ));
            }
        };

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(contents, uri)],
        })
    }

    fn read_readme(project_root: &PathBuf) -> Result<String, McpError> {
        let readme_path = project_root.join("README.md");
        std::fs::read_to_string(&readme_path)
            .map_err(|e| McpError::internal_error(
                format!("Failed to read README at {}: {}", readme_path.display(), e),
                None
            ))
    }

    fn generate_cli_reference() -> Result<String, McpError> {
        // Generate CLI reference documentation
        let mut output = String::from("# wasmcp CLI Reference\n\n");

        output.push_str("## Commands\n\n");

        output.push_str("### `wasmcp new`\n");
        output.push_str("Create a new MCP server handler component.\n\n");
        output.push_str("**Options:**\n");
        output.push_str("- `--language, -l <LANG>` - Programming language (rust, python, typescript)\n");
        output.push_str("- `--template-type, -t <TYPE>` - Template type (tools, resources, prompts)\n");
        output.push_str("- `--version` - wasmcp version to use for WIT dependencies\n");
        output.push_str("- `--force` - Overwrite existing directory\n");
        output.push_str("- `--output, -o <PATH>` - Output directory\n\n");

        output.push_str("### `wasmcp compose`\n");
        output.push_str("Compose handler components into a complete MCP server.\n\n");
        output.push_str("**Options:**\n");
        output.push_str("- `--profile, -p <NAME>` - Profile name from registry\n");
        output.push_str("- `--transport, -t <TYPE>` - Transport type (http, stdio)\n");
        output.push_str("- `--output, -o <PATH>` - Output path for composed server\n");
        output.push_str("- `--version` - wasmcp version for framework dependencies\n");
        output.push_str("- `--override-transport <SPEC>` - Override transport component\n");
        output.push_str("- `--override-method-not-found <SPEC>` - Override method-not-found component\n");
        output.push_str("- `--deps-dir <DIR>` - Directory for dependency components\n");
        output.push_str("- `--skip-download` - Skip downloading dependencies\n");
        output.push_str("- `--force` - Overwrite existing output file\n");
        output.push_str("- `--verbose, -v` - Enable verbose output\n\n");

        output.push_str("### `wasmcp wit fetch`\n");
        output.push_str("Fetch WIT dependencies for a project.\n\n");
        output.push_str("**Options:**\n");
        output.push_str("- `--dir <DIR>` - Directory containing wit/ folder (default: .)\n");
        output.push_str("- `--update` - Update dependencies to latest compatible versions\n\n");

        output.push_str("### `wasmcp registry component`\n");
        output.push_str("Manage component aliases.\n\n");
        output.push_str("**Subcommands:**\n");
        output.push_str("- `add <ALIAS> <SPEC>` - Register a component alias\n");
        output.push_str("- `remove <ALIAS>` - Unregister a component alias\n");
        output.push_str("- `list` - List registered component aliases\n\n");

        output.push_str("### `wasmcp registry profile`\n");
        output.push_str("Manage compose profiles.\n\n");
        output.push_str("**Subcommands:**\n");
        output.push_str("- `add <NAME> <COMPONENTS...> -o <OUTPUT>` - Create a new profile\n");
        output.push_str("- `remove <NAME>` - Delete a profile\n");
        output.push_str("- `list` - List all profiles\n\n");

        output.push_str("### `wasmcp registry info`\n");
        output.push_str("Show registry information, components, and profiles.\n\n");
        output.push_str("**Options:**\n");
        output.push_str("- `--components, -c` - Show only component aliases\n");
        output.push_str("- `--profiles, -p` - Show only profiles\n\n");

        output.push_str("### `wasmcp server`\n");
        output.push_str("Run MCP server for AI-assisted wasmcp development.\n\n");
        output.push_str("**Options:**\n");
        output.push_str("- `--port <PORT>` - Port for HTTP server (uses stdio if not specified)\n");
        output.push_str("- `--verbose, -v` - Enable verbose logging\n\n");

        Ok(output)
    }

    fn read_wit_protocol(project_root: &PathBuf) -> Result<String, McpError> {
        let wit_path = project_root.join("wit/protocol/mcp.wit");
        std::fs::read_to_string(&wit_path)
            .map_err(|e| McpError::internal_error(
                format!("Failed to read WIT at {}: {}", wit_path.display(), e),
                None
            ))
    }

    fn read_example_docs(project_root: &PathBuf, example_name: &str) -> Result<String, McpError> {
        let example_path = project_root.join("examples").join(example_name);

        if !example_path.exists() {
            return Err(McpError::resource_not_found(
                format!("Example '{}' not found", example_name),
                None
            ));
        }

        let mut output = format!("# {} Example\n\n", example_name);

        // Try to read README
        let readme_path = example_path.join("README.md");
        if readme_path.exists() {
            if let Ok(readme) = std::fs::read_to_string(&readme_path) {
                output.push_str(&readme);
                output.push_str("\n\n");
            }
        }

        // List source files
        output.push_str("## Source Files\n\n");
        if let Ok(entries) = std::fs::read_dir(&example_path.join("src")) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".rs") || name.ends_with(".py") || name.ends_with(".ts") {
                        output.push_str(&format!("- {}\n", name));
                    }
                }
            }
        }

        Ok(output)
    }

    fn format_registry_components(config: &WasmcpConfig) -> Result<String, McpError> {
        serde_json::to_string_pretty(&config.components)
            .map_err(|e| McpError::internal_error(
                format!("Failed to serialize components: {}", e),
                None
            ))
    }

    fn format_registry_profiles(config: &WasmcpConfig) -> Result<String, McpError> {
        serde_json::to_string_pretty(&config.profiles)
            .map_err(|e| McpError::internal_error(
                format!("Failed to serialize profiles: {}", e),
                None
            ))
    }

    fn read_registry_config(config: &WasmcpConfig) -> Result<String, McpError> {
        toml::to_string_pretty(&config)
            .map_err(|e| McpError::internal_error(
                format!("Failed to serialize config: {}", e),
                None
            ))
    }
}
