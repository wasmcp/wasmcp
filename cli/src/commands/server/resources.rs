use rmcp::ErrorData as McpError;
use rmcp::model::*;
use std::path::Path;

const GITHUB_RAW_BASE: &str = "https://raw.githubusercontent.com/wasmcp/wasmcp/main";

pub struct WasmcpResources;

impl WasmcpResources {
    pub fn list_all(_project_root: &Path) -> Result<ListResourcesResult, McpError> {
        let resources = vec![
            // Documentation resources from GitHub
            RawResource {
                uri: "wasmcp://docs/readme".into(),
                name: "wasmcp Project README".into(),
                mime_type: Some("text/markdown".into()),
                title: None,
                description: Some("Main project README from GitHub".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
            RawResource {
                uri: "wasmcp://docs/getting-started".into(),
                name: "Getting Started Guide".into(),
                mime_type: Some("text/markdown".into()),
                title: None,
                description: Some("Getting started with wasmcp".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
            RawResource {
                uri: "wasmcp://docs/wit-protocol".into(),
                name: "WIT Protocol Specification".into(),
                mime_type: Some("text/plain".into()),
                title: None,
                description: Some("MCP WIT interface definitions".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
            RawResource {
                uri: "wasmcp://docs/examples".into(),
                name: "Examples Overview".into(),
                mime_type: Some("text/markdown".into()),
                title: None,
                description: Some("Overview of example components".into()),
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
                description: Some("Component aliases from local wasmcp config".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
            RawResource {
                uri: "wasmcp://registry/profiles".into(),
                name: "Registry Composition Profiles".into(),
                mime_type: Some("application/json".into()),
                title: None,
                description: Some("Composition profiles from local wasmcp config".into()),
                size: None,
                icons: None,
            }
            .no_annotation(),
            RawResource {
                uri: "wasmcp://registry/config".into(),
                name: "Full Registry Configuration".into(),
                mime_type: Some("application/toml".into()),
                title: None,
                description: Some("Complete wasmcp.toml configuration".into()),
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
            // Documentation from GitHub
            "wasmcp://docs/readme" => Self::fetch_github_file(client, "README.md").await,
            "wasmcp://docs/getting-started" => {
                Self::fetch_github_file(client, "docs/getting-started.md").await
            }
            "wasmcp://docs/wit-protocol" => {
                Self::fetch_github_file(client, "wit/protocol/mcp.wit").await
            }
            "wasmcp://docs/examples" => Self::fetch_github_file(client, "examples/README.md").await,

            // Registry resources (local config - always read fresh from disk)
            "wasmcp://registry/components" => Self::read_components(),
            "wasmcp://registry/profiles" => Self::read_profiles(),
            "wasmcp://registry/config" => Self::read_config_toml().await,

            _ => Err(McpError::resource_not_found(uri.to_string(), None)),
        }
    }

    async fn fetch_github_file(
        client: &reqwest::Client,
        path: &str,
    ) -> Result<ReadResourceResult, McpError> {
        let url = format!("{}/{}", GITHUB_RAW_BASE, path);

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
