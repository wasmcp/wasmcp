mod runtime_impl;
pub mod tools;
pub mod deps;

use anyhow::Result;
use rmcp::{
    model::*,
    handler::server::ServerHandler,
    service::RequestContext,
    ErrorData as McpError,
    RoleServer,
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, StreamableHttpServerConfig,
    session::local::LocalSessionManager,
};
use runtime_impl::{NativeHttpClient, SystemTimeProvider, InMemoryCache, TracingLogger};
use std::sync::Arc;
use tracing_subscriber;

const BIND_ADDRESS: &str = "127.0.0.1:3000";

#[derive(Clone)]
pub struct WasmcpServer {
    // We'll store these for potential future use
    #[allow(dead_code)]
    http_client: NativeHttpClient,
    #[allow(dead_code)]
    time_provider: SystemTimeProvider,
    #[allow(dead_code)]
    cache: InMemoryCache,
    #[allow(dead_code)]
    logger: TracingLogger,
}

impl WasmcpServer {
    pub fn new() -> Self {
        Self {
            http_client: NativeHttpClient::new(),
            time_provider: SystemTimeProvider,
            cache: InMemoryCache::new(),
            logger: TracingLogger,
        }
    }
}

impl ServerHandler for WasmcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "wasmcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("WebAssembly MCP development tools. Tools: wasmcp_list, wasmcp_init, wasmcp_build, wasmcp_serve_spin, wasmcp_serve_wasmtime, wasmcp_compose, wasmcp_validate_wit, wasmcp_check_deps".to_string()),
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        Ok(self.get_info())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let all_tools = vec![
            Tool {
                name: "wasmcp_list".into(),
                description: Some("List all MCP provider projects in the workspace".into()),
                input_schema: Arc::new({
                    let mut schema = serde_json::Map::new();
                    schema.insert("type".to_string(), serde_json::json!("object"));
                    
                    let mut properties = serde_json::Map::new();
                    let mut path_schema = serde_json::Map::new();
                    path_schema.insert("type".to_string(), serde_json::json!("string"));
                    path_schema.insert("description".to_string(), serde_json::json!("Path to search for providers (defaults to current directory)"));
                    properties.insert("path".to_string(), serde_json::Value::Object(path_schema));
                    
                    schema.insert("properties".to_string(), serde_json::Value::Object(properties));
                    schema
                }),
                output_schema: None,
                annotations: None,
            },
            Tool {
                name: "wasmcp_init".into(),
                description: Some("Initialize a new MCP provider project".into()),
                input_schema: Arc::new({
                    let mut schema = serde_json::Map::new();
                    schema.insert("type".to_string(), serde_json::json!("object"));
                    
                    let mut properties = serde_json::Map::new();
                    
                    let mut language_schema = serde_json::Map::new();
                    language_schema.insert("type".to_string(), serde_json::json!("string"));
                    language_schema.insert("enum".to_string(), serde_json::json!(["rust", "python", "go", "typescript", "javascript"]));
                    language_schema.insert("description".to_string(), serde_json::json!("Programming language for the provider"));
                    properties.insert("language".to_string(), serde_json::Value::Object(language_schema));
                    
                    let mut name_schema = serde_json::Map::new();
                    name_schema.insert("type".to_string(), serde_json::json!("string"));
                    name_schema.insert("description".to_string(), serde_json::json!("Name of the new provider project"));
                    properties.insert("name".to_string(), serde_json::Value::Object(name_schema));
                    
                    schema.insert("properties".to_string(), serde_json::Value::Object(properties));
                    schema.insert("required".to_string(), serde_json::json!(["name"]));
                    schema
                }),
                output_schema: None,
                annotations: None,
            },
            Tool {
                name: "wasmcp_build".into(),
                description: Some("Build and compose provider with transport".into()),
                input_schema: Arc::new({
                    let mut schema = serde_json::Map::new();
                    schema.insert("type".to_string(), serde_json::json!("object"));
                    
                    let mut properties = serde_json::Map::new();
                    let mut path_schema = serde_json::Map::new();
                    path_schema.insert("type".to_string(), serde_json::json!("string"));
                    path_schema.insert("description".to_string(), serde_json::json!("Path to the provider project"));
                    properties.insert("path".to_string(), serde_json::Value::Object(path_schema));
                    
                    schema.insert("properties".to_string(), serde_json::Value::Object(properties));
                    schema
                }),
                output_schema: None,
                annotations: None,
            },
            Tool {
                name: "wasmcp_serve_spin".into(),
                description: Some("Serve composed WASM component using Spin runtime".into()),
                input_schema: Arc::new({
                    let mut schema = serde_json::Map::new();
                    schema.insert("type".to_string(), serde_json::json!("object"));
                    
                    let mut properties = serde_json::Map::new();
                    
                    let mut path_schema = serde_json::Map::new();
                    path_schema.insert("type".to_string(), serde_json::json!("string"));
                    path_schema.insert("description".to_string(), serde_json::json!("Path to the provider project (defaults to current directory)"));
                    properties.insert("path".to_string(), serde_json::Value::Object(path_schema));
                    
                    let mut composed_path_schema = serde_json::Map::new();
                    composed_path_schema.insert("type".to_string(), serde_json::json!("string"));
                    composed_path_schema.insert("description".to_string(), serde_json::json!("Path to composed WASM file (defaults to mcp-http-server.wasm)"));
                    properties.insert("composed_path".to_string(), serde_json::Value::Object(composed_path_schema));
                    
                    let mut port_schema = serde_json::Map::new();
                    port_schema.insert("type".to_string(), serde_json::json!("integer"));
                    port_schema.insert("description".to_string(), serde_json::json!("Port to serve on (defaults to 3001)"));
                    properties.insert("port".to_string(), serde_json::Value::Object(port_schema));
                    
                    schema.insert("properties".to_string(), serde_json::Value::Object(properties));
                    schema
                }),
                output_schema: None,
                annotations: None,
            },
            Tool {
                name: "wasmcp_serve_wasmtime".into(),
                description: Some("Serve composed WASM component using Wasmtime runtime".into()),
                input_schema: Arc::new({
                    let mut schema = serde_json::Map::new();
                    schema.insert("type".to_string(), serde_json::json!("object"));
                    
                    let mut properties = serde_json::Map::new();
                    
                    let mut path_schema = serde_json::Map::new();
                    path_schema.insert("type".to_string(), serde_json::json!("string"));
                    path_schema.insert("description".to_string(), serde_json::json!("Path to the provider project (defaults to current directory)"));
                    properties.insert("path".to_string(), serde_json::Value::Object(path_schema));
                    
                    let mut composed_path_schema = serde_json::Map::new();
                    composed_path_schema.insert("type".to_string(), serde_json::json!("string"));
                    composed_path_schema.insert("description".to_string(), serde_json::json!("Path to composed WASM file (defaults to mcp-http-server.wasm)"));
                    properties.insert("composed_path".to_string(), serde_json::Value::Object(composed_path_schema));
                    
                    let mut port_schema = serde_json::Map::new();
                    port_schema.insert("type".to_string(), serde_json::json!("integer"));
                    port_schema.insert("description".to_string(), serde_json::json!("Port to serve on (defaults to 3001)"));
                    properties.insert("port".to_string(), serde_json::Value::Object(port_schema));
                    
                    schema.insert("properties".to_string(), serde_json::Value::Object(properties));
                    schema
                }),
                output_schema: None,
                annotations: None,
            },
            Tool {
                name: "wasmcp_compose".into(),
                description: Some("Compose WASM components using wac".into()),
                input_schema: Arc::new({
                    let mut schema = serde_json::Map::new();
                    schema.insert("type".to_string(), serde_json::json!("object"));
                    
                    let mut properties = serde_json::Map::new();
                    
                    let mut provider_schema = serde_json::Map::new();
                    provider_schema.insert("type".to_string(), serde_json::json!("string"));
                    provider_schema.insert("description".to_string(), serde_json::json!("Path to provider component (defaults to target/wasm32-wasip1/release/provider.wasm)"));
                    properties.insert("provider".to_string(), serde_json::Value::Object(provider_schema));
                    
                    let mut transport_schema = serde_json::Map::new();
                    transport_schema.insert("type".to_string(), serde_json::json!("string"));
                    transport_schema.insert("description".to_string(), serde_json::json!("Transport component to use (defaults to wasmcp:mcp-transport-http@0.1.0)"));
                    properties.insert("transport".to_string(), serde_json::Value::Object(transport_schema));
                    
                    let mut output_schema = serde_json::Map::new();
                    output_schema.insert("type".to_string(), serde_json::json!("string"));
                    output_schema.insert("description".to_string(), serde_json::json!("Output file name (defaults to mcp-http-server.wasm)"));
                    properties.insert("output".to_string(), serde_json::Value::Object(output_schema));
                    
                    let mut path_schema = serde_json::Map::new();
                    path_schema.insert("type".to_string(), serde_json::json!("string"));
                    path_schema.insert("description".to_string(), serde_json::json!("Project path (defaults to current directory)"));
                    properties.insert("path".to_string(), serde_json::Value::Object(path_schema));
                    
                    schema.insert("properties".to_string(), serde_json::Value::Object(properties));
                    schema
                }),
                output_schema: None,
                annotations: None,
            },
            Tool {
                name: "wasmcp_validate_wit".into(),
                description: Some("Validate WIT interface definitions".into()),
                input_schema: Arc::new({
                    let mut schema = serde_json::Map::new();
                    schema.insert("type".to_string(), serde_json::json!("object"));
                    
                    let mut properties = serde_json::Map::new();
                    
                    let mut path_schema = serde_json::Map::new();
                    path_schema.insert("type".to_string(), serde_json::json!("string"));
                    path_schema.insert("description".to_string(), serde_json::json!("Path to WIT directory (defaults to 'wit')"));
                    properties.insert("path".to_string(), serde_json::Value::Object(path_schema));
                    
                    let mut project_path_schema = serde_json::Map::new();
                    project_path_schema.insert("type".to_string(), serde_json::json!("string"));
                    project_path_schema.insert("description".to_string(), serde_json::json!("Project path (defaults to current directory)"));
                    properties.insert("project_path".to_string(), serde_json::Value::Object(project_path_schema));
                    
                    schema.insert("properties".to_string(), serde_json::Value::Object(properties));
                    schema
                }),
                output_schema: None,
                annotations: None,
            },
            Tool {
                name: "wasmcp_check_deps".into(),
                description: Some("Check external dependencies and tool availability".into()),
                input_schema: Arc::new({
                    let mut schema = serde_json::Map::new();
                    schema.insert("type".to_string(), serde_json::json!("object"));
                    schema.insert("properties".to_string(), serde_json::json!({}));
                    schema
                }),
                output_schema: None,
                annotations: None,
            },
        ];

        // Filter to only available tools
        let tools = all_tools
            .into_iter()
            .filter(|tool| deps::is_tool_available(&tool.name))
            .collect();

        Ok(ListToolsResult { 
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let args = request.arguments.map(|m| serde_json::Value::Object(m));
        
        match request.name.as_ref() {
            "wasmcp_list" => tools::project::wasmcp_list(args)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None)),
            "wasmcp_init" => tools::project::wasmcp_init(args)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None)),
            "wasmcp_build" => tools::build::wasmcp_build(args)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None)),
            "wasmcp_serve_spin" => tools::serve::wasmcp_serve_spin(args)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None)),
            "wasmcp_serve_wasmtime" => tools::serve::wasmcp_serve_wasmtime(args)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None)),
            "wasmcp_compose" => tools::compose::wasmcp_compose(args)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None)),
            "wasmcp_validate_wit" => tools::compose::wasmcp_validate_wit(args)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None)),
            "wasmcp_check_deps" => tools::deps::wasmcp_check_deps(args)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None)),
            _ => Err(McpError::invalid_params(format!("Unknown tool: {}", request.name), None)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    // Check dependencies at startup
    let (installed, missing) = deps::check_all_dependencies();
    tracing::info!("Dependency check - Installed: {:?}", installed);
    if !missing.is_empty() {
        tracing::warn!("Missing dependencies: {:?}", missing);
        tracing::warn!("Some tools may be unavailable. Run wasmcp_check_deps for details.");
    }

    let config = StreamableHttpServerConfig {
        stateful_mode: true,
        sse_keep_alive: Some(std::time::Duration::from_secs(15)),
    };
    
    let service = StreamableHttpService::new(
        || Ok(WasmcpServer::new()),
        LocalSessionManager::default().into(),
        config,
    );

    // Mount at /mcp endpoint
    let app = axum::Router::new().nest_service("/mcp", service);

    let listener = tokio::net::TcpListener::bind(BIND_ADDRESS).await?;
    tracing::info!("wasmcp MCP server listening on {}", BIND_ADDRESS);
    
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
            tracing::info!("Shutting down server");
        })
        .await?;

    Ok(())
}