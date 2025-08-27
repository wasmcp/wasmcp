use spin_sdk::http::{IntoResponse, Request, Response};
use anyhow::Result;
use rmcp::model::{
    CallToolRequestParam, ListToolsResult, ServerInfo,
    ListPromptsResult, ListResourcesResult,
    PaginatedRequestParam, GetPromptRequestParam, ReadResourceRequestParam,
    InitializeRequestParam,
};

// cargo-component will generate bindings automatically
#[allow(warnings)]
mod bindings;

mod adapter;
use adapter::WitMcpAdapter;

// Import the MCP handler for notifications
use bindings::fastertools::mcp::core;

/// MCP Server that bridges to WIT interface
struct McpServer {
    adapter: WitMcpAdapter,
}

impl McpServer {
    fn new() -> Self {
        Self {
            adapter: WitMcpAdapter::new(),
        }
    }
    
    fn get_server_info(&self) -> Result<ServerInfo> {
        self.adapter.get_server_info()
    }
}

#[spin_sdk::http_component]
async fn handle_request(req: Request) -> Result<impl IntoResponse> {
    let body = req.body();
    let request_str = std::str::from_utf8(body)?;
    
    // Create the MCP server
    let server = McpServer::new();
    
    // Parse JSON-RPC request
    let json_request: serde_json::Value = serde_json::from_str(request_str)?;
    
    // Extract method and handle accordingly
    let method = json_request.get("method")
        .and_then(|m| m.as_str())
        .unwrap_or("");
    
    let id = json_request.get("id").cloned();
    let params = json_request.get("params").cloned();
    
    // Route to appropriate handler using rmcp types for protocol compliance
    let result = match method {
        "initialize" => {
            let _params: InitializeRequestParam = if let Some(p) = params {
                serde_json::from_value(p)?
            } else {
                InitializeRequestParam::default()
            };
            
            let server_info = server.get_server_info()?;
            Ok(serde_json::to_value(server_info)?)
        },
        "initialized" | "notifications/initialized" => {
            // Handle initialized notification
            core::handle_initialized().ok();
            // Notification, return empty body with 200
            return Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body("")
                .build());
        },
        "tools/list" => {
            let _params: Option<PaginatedRequestParam> = params
                .map(|p| serde_json::from_value(p))
                .transpose()?;
            
            let tools = server.adapter.list_tools().await?;
            let result = ListToolsResult {
                tools,
                next_cursor: None,
            };
            Ok(serde_json::to_value(result)?)
        },
        "tools/call" => {
            let params: CallToolRequestParam = serde_json::from_value(params.unwrap_or_default())?;
            let result = server.adapter.call_tool(&params.name, params.arguments).await?;
            Ok(serde_json::to_value(result)?)
        },
        "resources/list" => {
            let _params: Option<PaginatedRequestParam> = params
                .map(|p| serde_json::from_value(p))
                .transpose()?;
            
            let resources = server.adapter.list_resources().await?;
            let result = ListResourcesResult {
                resources,
                next_cursor: None,
            };
            Ok(serde_json::to_value(result)?)
        },
        "resources/read" => {
            let params: ReadResourceRequestParam = serde_json::from_value(params.unwrap_or_default())?;
            let result = server.adapter.read_resource(&params.uri).await?;
            Ok(serde_json::to_value(result)?)
        },
        "prompts/list" => {
            let _params: Option<PaginatedRequestParam> = params
                .map(|p| serde_json::from_value(p))
                .transpose()?;
            
            let prompts = server.adapter.list_prompts().await?;
            let result = ListPromptsResult {
                prompts,
                next_cursor: None,
            };
            Ok(serde_json::to_value(result)?)
        },
        "prompts/get" => {
            let params: GetPromptRequestParam = serde_json::from_value(params.unwrap_or_default())?;
            let result = server.adapter.get_prompt(&params.name, params.arguments).await?;
            Ok(serde_json::to_value(result)?)
        },
        "ping" => {
            core::handle_ping().ok();
            Ok(serde_json::json!({}))
        },
        "shutdown" => {
            core::handle_shutdown().ok();
            Ok(serde_json::json!({}))
        },
        _ => {
            // Check if it's another notification (no id)
            if id.is_none() {
                // Unknown notification, just acknowledge with 204
                return Ok(Response::builder()
                    .status(204)
                    .build());
            }
            Err(anyhow::anyhow!("Method not found: {}", method))
        }
    };
    
    // Build JSON-RPC response
    let response_json = match result {
        Ok(result) => {
            serde_json::json!({
                "jsonrpc": "2.0",
                "result": result,
                "id": id
            })
        },
        Err(e) => {
            serde_json::json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32601,
                    "message": e.to_string()
                },
                "id": id
            })
        }
    };
    
    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(serde_json::to_string(&response_json)?)
        .build())
}