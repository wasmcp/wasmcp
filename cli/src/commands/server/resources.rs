use rmcp::ErrorData as McpError;
use rmcp::model::*;
use std::path::Path;

const GITHUB_REPO: &str = "https://raw.githubusercontent.com/wasmcp/wasmcp";
const DEFAULT_BRANCH: &str = "main";

pub struct WasmcpResources;

impl WasmcpResources {
    pub fn list_templates() -> Result<ListResourceTemplatesResult, McpError> {
        let resource_templates = vec![
            // Documentation templates
            RawResourceTemplate {
                uri_template: "wasmcp://branch/{branch}/docs/{resource}".into(),
                name: "Branch-specific Documentation".into(),
                title: None,
                description: Some(
                    "Access documentation from specific Git branches (e.g., develop, v0.4.0). Available resources: readme, getting-started, cli, architecture, examples, wit-interfaces"
                        .into(),
                ),
                mime_type: Some("text/markdown".into()),
            }
            .no_annotation(),
            // WIT protocol templates
            RawResourceTemplate {
                uri_template: "wasmcp://branch/{branch}/wit/protocol/{resource}".into(),
                name: "Branch-specific WIT Protocol Interfaces".into(),
                title: None,
                description: Some(
                    "Access WIT protocol interfaces from specific Git branches. Available resources: mcp, features"
                        .into(),
                ),
                mime_type: Some("text/plain".into()),
            }
            .no_annotation(),
            // WIT server templates
            RawResourceTemplate {
                uri_template: "wasmcp://branch/{branch}/wit/server/{resource}".into(),
                name: "Branch-specific WIT Server Interfaces".into(),
                title: None,
                description: Some(
                    "Access WIT server interfaces from specific Git branches. Available resources: handler, sessions, notifications"
                        .into(),
                ),
                mime_type: Some("text/plain".into()),
            }
            .no_annotation(),
        ];

        Ok(ListResourceTemplatesResult {
            resource_templates,
            next_cursor: None,
        })
    }

    pub fn list_all(_project_root: &Path) -> Result<ListResourcesResult, McpError> {
        let resources = vec![
            // Documentation resources from GitHub
            RawResource {
                uri: "wasmcp://docs/readme".into(),
                name: "wasmcp Project README".into(),
                mime_type: Some("text/markdown".into()),
                title: None,
                description: Some("Main project overview and quick start guide".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
            RawResource {
                uri: "wasmcp://docs/getting-started".into(),
                name: "Getting Started Guide".into(),
                mime_type: Some("text/markdown".into()),
                title: None,
                description: Some("Step-by-step tutorial for creating your first MCP component with wasmcp".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
            RawResource {
                uri: "wasmcp://docs/cli".into(),
                name: "CLI Reference".into(),
                mime_type: Some("text/markdown".into()),
                title: None,
                description: Some("Detailed CLI command documentation and advanced usage".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
            RawResource {
                uri: "wasmcp://docs/architecture".into(),
                name: "Architecture Guide".into(),
                mime_type: Some("text/markdown".into()),
                title: None,
                description: Some("Component model, composition pipeline, and capability/middleware pattern explanation".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
            RawResource {
                uri: "wasmcp://docs/examples".into(),
                name: "Examples Overview".into(),
                mime_type: Some("text/markdown".into()),
                title: None,
                description: Some("Overview of example components (calculator-rs, strings-py, weather-ts) with learning path".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
            RawResource {
                uri: "wasmcp://docs/wit-interfaces".into(),
                name: "WIT Interface Reference".into(),
                mime_type: Some("text/markdown".into()),
                title: None,
                description: Some("Complete WIT interface documentation with protocol and server package overview".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
            // WIT protocol interfaces from GitHub
            RawResource {
                uri: "wasmcp://wit/protocol/mcp".into(),
                name: "MCP Protocol WIT".into(),
                mime_type: Some("text/plain".into()),
                title: None,
                description: Some("Complete MCP protocol type definitions (JSON-RPC, requests, responses, errors)".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
            RawResource {
                uri: "wasmcp://wit/protocol/features".into(),
                name: "MCP Features WIT".into(),
                mime_type: Some("text/plain".into()),
                title: None,
                description: Some("MCP capability interfaces (tools, resources, prompts) that components export".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
            RawResource {
                uri: "wasmcp://wit/server/handler".into(),
                name: "Server Handler WIT".into(),
                mime_type: Some("text/plain".into()),
                title: None,
                description: Some("Core request handler interface used by middleware and transport components".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
            RawResource {
                uri: "wasmcp://wit/server/sessions".into(),
                name: "Server Sessions WIT".into(),
                mime_type: Some("text/plain".into()),
                title: None,
                description: Some("Session management interfaces for stateful middleware components".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
            RawResource {
                uri: "wasmcp://wit/server/notifications".into(),
                name: "Server Notifications WIT".into(),
                mime_type: Some("text/plain".into()),
                title: None,
                description: Some("Server-to-client notification interfaces (progress, logs, resource updates)".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
            // Registry resources (local config)
            RawResource {
                uri: "wasmcp://registry/components".into(),
                name: "Registry Component Aliases".into(),
                mime_type: Some("application/json".into()),
                title: None,
                description: Some("Component aliases from local wasmcp config (JSON format)".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
            RawResource {
                uri: "wasmcp://registry/profiles".into(),
                name: "Registry Composition Profiles".into(),
                mime_type: Some("application/json".into()),
                title: None,
                description: Some("Composition profiles from local wasmcp config (JSON format)".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
            RawResource {
                uri: "wasmcp://registry/config".into(),
                name: "Full Registry Configuration".into(),
                mime_type: Some("application/toml".into()),
                title: None,
                description: Some("Complete wasmcp.toml configuration file".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
        ];

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
        })
    }

    pub async fn read(
        client: &reqwest::Client,
        uri: &str,
        _project_root: &Path,
    ) -> Result<ReadResourceResult, McpError> {
        match uri {
            // Documentation from GitHub (main branch)
            "wasmcp://docs/readme" => {
                Self::fetch_github_file(client, DEFAULT_BRANCH, "README.md").await
            }
            "wasmcp://docs/getting-started" => {
                Self::fetch_github_file(client, DEFAULT_BRANCH, "docs/getting-started.md").await
            }
            "wasmcp://docs/cli" => {
                Self::fetch_github_file(client, DEFAULT_BRANCH, "cli/README.md").await
            }
            "wasmcp://docs/architecture" => {
                Self::fetch_github_file(client, DEFAULT_BRANCH, "docs/architecture.md").await
            }
            "wasmcp://docs/examples" => {
                Self::fetch_github_file(client, DEFAULT_BRANCH, "docs/examples.md").await
            }
            "wasmcp://docs/wit-interfaces" => {
                Self::fetch_github_file(client, DEFAULT_BRANCH, "docs/wit-interfaces.md").await
            }

            // WIT protocol interfaces from GitHub (main branch)
            "wasmcp://wit/protocol/mcp" => {
                Self::fetch_github_file(client, DEFAULT_BRANCH, "wit/protocol/mcp.wit").await
            }
            "wasmcp://wit/protocol/features" => {
                Self::fetch_github_file(client, DEFAULT_BRANCH, "wit/protocol/features.wit").await
            }
            "wasmcp://wit/server/handler" => {
                Self::fetch_github_file(client, DEFAULT_BRANCH, "wit/server/handler.wit").await
            }
            "wasmcp://wit/server/sessions" => {
                Self::fetch_github_file(client, DEFAULT_BRANCH, "wit/server/sessions.wit").await
            }
            "wasmcp://wit/server/notifications" => {
                Self::fetch_github_file(client, DEFAULT_BRANCH, "wit/server/notifications.wit")
                    .await
            }

            // Registry resources (local config - always read fresh from disk)
            "wasmcp://registry/components" => Self::read_components(),
            "wasmcp://registry/profiles" => Self::read_profiles(),
            "wasmcp://registry/config" => Self::read_config_toml().await,

            // Template-based branch resources
            _ if uri.starts_with("wasmcp://branch/") => {
                Self::read_branch_resource(client, uri).await
            }

            _ => Err(McpError::resource_not_found(uri.to_string(), None)),
        }
    }

    /// Parse and fetch a branch-specific resource from template URI
    async fn read_branch_resource(
        client: &reqwest::Client,
        uri: &str,
    ) -> Result<ReadResourceResult, McpError> {
        // Parse URI pattern: wasmcp://branch/{branch}/{namespace}/{resource}
        let parts: Vec<&str> = uri.strip_prefix("wasmcp://branch/")
            .ok_or_else(|| McpError::invalid_params(
                format!("Invalid branch resource URI: {}", uri),
                None,
            ))?
            .split('/')
            .collect();

        if parts.len() < 3 {
            return Err(McpError::invalid_params(
                format!(
                    "Branch resource URI must be wasmcp://branch/{{branch}}/{{namespace}}/{{resource}}, got: {}",
                    uri
                ),
                None,
            ));
        }

        let branch = parts[0];
        let namespace = parts[1];
        let resource = &parts[2..].join("/");

        // Validate branch name (basic security check)
        if !branch.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_') {
            return Err(McpError::invalid_params(
                format!("Invalid branch name: {}", branch),
                None,
            ));
        }

        // Map namespace and resource to file path
        let file_path = match namespace {
            "docs" => match resource.as_str() {
                "readme" => "README.md",
                "getting-started" => "docs/getting-started.md",
                "cli" => "cli/README.md",
                "architecture" => "docs/architecture.md",
                "examples" => "docs/examples.md",
                "wit-interfaces" => "docs/wit-interfaces.md",
                _ => return Err(McpError::resource_not_found(uri.to_string(), None)),
            },
            "wit" => {
                if parts.len() < 4 {
                    return Err(McpError::invalid_params(
                        format!("WIT resource URI must include protocol or server: {}", uri),
                        None,
                    ));
                }
                let wit_namespace = parts[2]; // "protocol" or "server"
                let wit_resource = parts[3];
                match (wit_namespace, wit_resource) {
                    ("protocol", "mcp") => "wit/protocol/mcp.wit",
                    ("protocol", "features") => "wit/protocol/features.wit",
                    ("server", "handler") => "wit/server/handler.wit",
                    ("server", "sessions") => "wit/server/sessions.wit",
                    ("server", "notifications") => "wit/server/notifications.wit",
                    _ => return Err(McpError::resource_not_found(uri.to_string(), None)),
                }
            }
            _ => return Err(McpError::resource_not_found(uri.to_string(), None)),
        };

        Self::fetch_github_file(client, branch, file_path).await
    }

    async fn fetch_github_file(
        client: &reqwest::Client,
        branch: &str,
        path: &str,
    ) -> Result<ReadResourceResult, McpError> {
        let url = format!("{}/{}/{}", GITHUB_REPO, branch, path);

        let response = client.get(&url).send().await.map_err(|e| {
            McpError::internal_error(
                format!("Failed to fetch from GitHub: {}", e),
                Some(serde_json::json!({
                    "url": url,
                    "path": path,
                    "error": e.to_string(),
                })),
            )
        })?;

        if !response.status().is_success() {
            return Err(McpError::internal_error(
                format!("GitHub returned status {}: {}", response.status(), url),
                Some(serde_json::json!({
                    "url": url,
                    "status_code": response.status().as_u16(),
                    "status": response.status().to_string(),
                })),
            ));
        }

        let content = response.text().await.map_err(|e| {
            McpError::internal_error(
                format!("Failed to read response: {}", e),
                Some(serde_json::json!({
                    "url": url,
                    "error": e.to_string(),
                })),
            )
        })?;

        let uri_str = format!("wasmcp://docs/{}", path.replace('/', "-"));

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(content, uri_str)],
        })
    }

    fn read_components() -> Result<ReadResourceResult, McpError> {
        // Load fresh config from disk
        let config = crate::config::load_config()
            .map_err(|e| McpError::internal_error(format!("Failed to load config: {}", e), None))?;

        let components_json = serde_json::to_string_pretty(&config.components).map_err(|e| {
            McpError::internal_error(format!("Failed to serialize components: {}", e), None)
        })?;

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(
                components_json,
                "wasmcp://registry/components".to_string(),
            )],
        })
    }

    fn read_profiles() -> Result<ReadResourceResult, McpError> {
        // Load fresh config from disk
        let config = crate::config::load_config()
            .map_err(|e| McpError::internal_error(format!("Failed to load config: {}", e), None))?;

        let profiles_json = serde_json::to_string_pretty(&config.profiles).map_err(|e| {
            McpError::internal_error(format!("Failed to serialize profiles: {}", e), None)
        })?;

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(
                profiles_json,
                "wasmcp://registry/profiles".to_string(),
            )],
        })
    }

    async fn read_config_toml() -> Result<ReadResourceResult, McpError> {
        let config_path = crate::config::get_config_path().map_err(|e| {
            McpError::internal_error(format!("Failed to get config path: {}", e), None)
        })?;

        let config_content = tokio::fs::read_to_string(&config_path).await.map_err(|e| {
            McpError::internal_error(format!("Failed to read config file: {}", e), None)
        })?;

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(
                config_content,
                "wasmcp://registry/config".to_string(),
            )],
        })
    }
}
