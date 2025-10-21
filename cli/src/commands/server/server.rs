use anyhow::Result;
use rmcp::model::*;
use rmcp::service::{RequestContext, RoleServer};
use rmcp::transport::stdio;
use rmcp::ErrorData as McpError;
use rmcp::{ServerHandler, ServiceExt};
use std::path::PathBuf;
use std::sync::Arc;

use super::resources::WasmcpResources;
use super::tools::*;
use crate::config::{load_config, WasmcpConfig};

#[derive(Clone)]
pub struct WasmcpServer {
    config: WasmcpConfig,
    project_root: PathBuf,
}

impl WasmcpServer {
    pub fn new(project_root: PathBuf) -> Result<Self> {
        let config = load_config()?;

        Ok(Self {
            config,
            project_root,
        })
    }
}

impl ServerHandler for WasmcpServer {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::V_2024_11_05,
            server_info: Implementation {
                name: "wasmcp-mcp-server".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: Some("wasmcp MCP Server".into()),
                icons: None,
                website_url: Some("https://github.com/wasmcp/wasmcp".into()),
            },
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            instructions: Some(
                "MCP server providing wasmcp documentation, registry management, \
                 and composition tools for AI-assisted development."
                    .into(),
            ),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        WasmcpResources::list_all(&self.project_root)
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        WasmcpResources::read(&request.uri, &self.project_root, &self.config)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let schema_compose = serde_json::json!({
            "type": "object",
            "properties": {
                "components": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Components to compose (profiles, aliases, or paths)"
                },
                "output": {
                    "type": "string",
                    "description": "Output file path"
                },
                "transport": {
                    "type": "string",
                    "description": "Transport type (http or stdio)"
                }
            },
            "required": ["components"]
        });

        let schema_list = serde_json::json!({
            "type": "object",
            "properties": {
                "target": {
                    "type": "string",
                    "description": "What to list (components, profiles, or all)",
                    "default": "all"
                }
            }
        });

        let schema_add_component = serde_json::json!({
            "type": "object",
            "properties": {
                "alias": {
                    "type": "string",
                    "description": "Component alias name"
                },
                "spec": {
                    "type": "string",
                    "description": "Component path or reference"
                }
            },
            "required": ["alias", "spec"]
        });

        let schema_add_profile = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Profile name"
                },
                "components": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Components in profile"
                },
                "output": {
                    "type": "string",
                    "description": "Output path"
                }
            },
            "required": ["name", "components"]
        });

        let schema_remove = serde_json::json!({
            "type": "object",
            "properties": {
                "kind": {
                    "type": "string",
                    "description": "Type to remove (component or profile)"
                },
                "name": {
                    "type": "string",
                    "description": "Name to remove"
                }
            },
            "required": ["kind", "name"]
        });

        Ok(ListToolsResult {
            tools: vec![
                Tool {
                    name: "compose".into(),
                    title: None,
                    description: Some("Compose WASM components into an MCP server".into()),
                    input_schema: Arc::new(serde_json::from_value(schema_compose).unwrap()),
                    output_schema: None,
                    annotations: None,
                    icons: None,
                },
                Tool {
                    name: "registry_list".into(),
                    title: None,
                    description: Some("List registry components, profiles, and aliases".into()),
                    input_schema: Arc::new(serde_json::from_value(schema_list).unwrap()),
                    output_schema: None,
                    annotations: None,
                    icons: None,
                },
                Tool {
                    name: "registry_add_component".into(),
                    title: None,
                    description: Some("Add a component alias to the registry".into()),
                    input_schema: Arc::new(serde_json::from_value(schema_add_component).unwrap()),
                    output_schema: None,
                    annotations: None,
                    icons: None,
                },
                Tool {
                    name: "registry_add_profile".into(),
                    title: None,
                    description: Some("Add or update a composition profile".into()),
                    input_schema: Arc::new(serde_json::from_value(schema_add_profile).unwrap()),
                    output_schema: None,
                    annotations: None,
                    icons: None,
                },
                Tool {
                    name: "registry_remove".into(),
                    title: None,
                    description: Some("Remove a component alias or profile".into()),
                    input_schema: Arc::new(serde_json::from_value(schema_remove).unwrap()),
                    output_schema: None,
                    annotations: None,
                    icons: None,
                },
            ],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let args_value = serde_json::Value::Object(request.arguments.unwrap_or_default());

        match request.name.as_ref() {
            "compose" => {
                let args: ComposeArgs = serde_json::from_value(args_value)
                    .map_err(|e| McpError::invalid_params(format!("Invalid arguments: {}", e), None))?;
                compose_tool(args).await
            }
            "registry_list" => {
                let args: RegistryListArgs = serde_json::from_value(args_value)
                    .map_err(|e| McpError::invalid_params(format!("Invalid arguments: {}", e), None))?;
                registry_list_tool(&self.config, args).await
            }
            "registry_add_component" => {
                let args: AddComponentArgs = serde_json::from_value(args_value)
                    .map_err(|e| McpError::invalid_params(format!("Invalid arguments: {}", e), None))?;
                registry_add_component_tool(args).await
            }
            "registry_add_profile" => {
                let args: AddProfileArgs = serde_json::from_value(args_value)
                    .map_err(|e| McpError::invalid_params(format!("Invalid arguments: {}", e), None))?;
                registry_add_profile_tool(args).await
            }
            "registry_remove" => {
                let args: RemoveArgs = serde_json::from_value(args_value)
                    .map_err(|e| McpError::invalid_params(format!("Invalid arguments: {}", e), None))?;
                registry_remove_tool(args).await
            }
            _ => Err(McpError::method_not_found::<CallToolRequestMethod>()),
        }
    }
}

pub async fn start_server(port: Option<u16>, verbose: bool) -> Result<()> {
    if verbose {
        tracing_subscriber::fmt()
            .with_target(false)
            .with_level(true)
            .init();
    }

    let project_root = std::env::current_dir()?;
    let server = WasmcpServer::new(project_root)?;

    match port {
        None => {
            eprintln!("Starting wasmcp MCP server (stdio mode)...");
            let service = server.serve(stdio()).await?;
            service.waiting().await?;
        }
        Some(port) => {
            start_http_server(server, port).await?;
        }
    }

    Ok(())
}

async fn start_http_server(server: WasmcpServer, port: u16) -> Result<()> {
    use rmcp::transport::streamable_http_server::{
        StreamableHttpService, session::local::LocalSessionManager,
    };
    use std::net::SocketAddr;

    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse()?;

    eprintln!("Starting wasmcp MCP server (HTTP/Streamable mode)...");
    eprintln!("Listening on http://{}", addr);
    eprintln!("MCP endpoint: http://{}/mcp", addr);

    // Create streamable HTTP service
    let service = StreamableHttpService::new(
        move || Ok(server.clone()),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    // Create router with single /mcp endpoint
    let router = axum::Router::new().nest_service("/mcp", service);

    // Start the HTTP server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            eprintln!("Received shutdown signal...");
        })
        .await?;

    Ok(())
}
