use spin_sdk::http::{IntoResponse, Request, Response};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// Bindings generated from WIT by cargo-component
#[allow(warnings)]
mod bindings;

// Import the MCP protocol types and handler interfaces
use bindings::fastertools::mcp::{
    core,
    tool_handler,
    resource_handler,
    prompt_handler,
    session::InitializeRequest,
    tools::{ListToolsRequest, CallToolRequest},
    resources::{ListResourcesRequest, ReadResourceRequest},
    prompts::{ListPromptsRequest, GetPromptRequest},
    types::{McpError, ErrorCode},
};

/// JSON-RPC 2.0 Request
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: Option<Value>,
    id: Option<Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    id: Option<Value>,
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl From<McpError> for JsonRpcError {
    fn from(err: McpError) -> Self {
        let code = match err.code {
            ErrorCode::ParseError => -32700,
            ErrorCode::InvalidRequest => -32600,
            ErrorCode::MethodNotFound => -32601,
            ErrorCode::InvalidParams => -32602,
            ErrorCode::InternalError => -32603,
            _ => -32000, // Generic error code for MCP-specific errors
        };
        
        JsonRpcError {
            code,
            message: err.message,
            data: err.data.and_then(|s: String| serde_json::from_str(&s).ok()),
        }
    }
}

/// Main HTTP handler for the MCP server
#[spin_sdk::http_component]
async fn handle_request(req: Request) -> Result<impl IntoResponse> {
    // Parse the request body as JSON-RPC
    let body = req.body();
    let request: JsonRpcRequest = match serde_json::from_slice(body) {
        Ok(req) => req,
        Err(e) => {
            return Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(serde_json::to_string(&JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                    id: None,
                })?)
                .build());
        }
    };
    
    // Route the request to the appropriate handler
    let result = match request.method.as_str() {
        // Core handlers (always available)
        "initialize" => handle_initialize(request.params),
        "initialized" => handle_initialized(),
        "ping" => handle_ping(),
        "shutdown" => handle_shutdown(),
        
        // Tool handlers (check if available)
        "tools/list" => handle_list_tools(request.params),
        "tools/call" => handle_call_tool(request.params),
        
        // Resource handlers (check if available)
        "resources/list" => handle_list_resources(request.params),
        "resources/read" => handle_read_resource(request.params),
        
        // Prompt handlers (check if available)
        "prompts/list" => handle_list_prompts(request.params),
        "prompts/get" => handle_get_prompt(request.params),
        
        _ => Err(McpError {
            code: ErrorCode::MethodNotFound,
            message: format!("Method not found: {}", request.method),
            data: None,
        }),
    };
    
    // Build the JSON-RPC response
    let response = match result {
        Ok(value) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(value),
            error: None,
            id: request.id,
        },
        Err(e) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(e.into()),
            id: request.id,
        },
    };
    
    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(serde_json::to_string(&response)?)
        .build())
}

// Core handlers - always available

fn handle_initialize(params: Option<Value>) -> Result<Value, McpError> {
    let request: InitializeRequest = if let Some(p) = params {
        serde_json::from_value(p).map_err(|e| McpError {
            code: ErrorCode::InvalidParams,
            message: format!("Invalid params: {}", e),
            data: None,
        })?
    } else {
        return Err(McpError {
            code: ErrorCode::InvalidParams,
            message: "Missing params".to_string(),
            data: None,
        });
    };
    
    let response = core::handle_initialize(&request)?;
    
    serde_json::to_value(response).map_err(|e| McpError {
        code: ErrorCode::InternalError,
        message: format!("Serialization error: {}", e),
        data: None,
    })
}

fn handle_initialized() -> Result<Value, McpError> {
    core::handle_initialized()?;
    Ok(json!({}))
}

fn handle_ping() -> Result<Value, McpError> {
    core::handle_ping()?;
    Ok(json!({}))
}

fn handle_shutdown() -> Result<Value, McpError> {
    core::handle_shutdown()?;
    Ok(json!({}))
}

// Tool handlers - may not be available

fn handle_list_tools(params: Option<Value>) -> Result<Value, McpError> {
    // Check if tool handler is available (for now we assume it is)
    let request: ListToolsRequest = if let Some(p) = params {
        serde_json::from_value(p).map_err(|e| McpError {
            code: ErrorCode::InvalidParams,
            message: format!("Invalid params: {}", e),
            data: None,
        })?
    } else {
        ListToolsRequest {
            cursor: None,
            progress_token: None,
            meta: None,
        }
    };
    
    let response = tool_handler::handle_list_tools(&request)?;
    
    serde_json::to_value(response).map_err(|e| McpError {
        code: ErrorCode::InternalError,
        message: format!("Serialization error: {}", e),
        data: None,
    })
}

fn handle_call_tool(params: Option<Value>) -> Result<Value, McpError> {
    let request: CallToolRequest = if let Some(p) = params {
        serde_json::from_value(p).map_err(|e| McpError {
            code: ErrorCode::InvalidParams,
            message: format!("Invalid params: {}", e),
            data: None,
        })?
    } else {
        return Err(McpError {
            code: ErrorCode::InvalidParams,
            message: "Missing params".to_string(),
            data: None,
        });
    };
    
    let response = tool_handler::handle_call_tool(&request)?;
    
    serde_json::to_value(response).map_err(|e| McpError {
        code: ErrorCode::InternalError,
        message: format!("Serialization error: {}", e),
        data: None,
    })
}

// Resource handlers - may not be available

fn handle_list_resources(params: Option<Value>) -> Result<Value, McpError> {
    let request: ListResourcesRequest = if let Some(p) = params {
        serde_json::from_value(p).map_err(|e| McpError {
            code: ErrorCode::InvalidParams,
            message: format!("Invalid params: {}", e),
            data: None,
        })?
    } else {
        ListResourcesRequest {
            cursor: None,
            progress_token: None,
            meta: None,
        }
    };
    
    let response = resource_handler::handle_list_resources(&request)?;
    
    serde_json::to_value(response).map_err(|e| McpError {
        code: ErrorCode::InternalError,
        message: format!("Serialization error: {}", e),
        data: None,
    })
}

fn handle_read_resource(params: Option<Value>) -> Result<Value, McpError> {
    let request: ReadResourceRequest = if let Some(p) = params {
        serde_json::from_value(p).map_err(|e| McpError {
            code: ErrorCode::InvalidParams,
            message: format!("Invalid params: {}", e),
            data: None,
        })?
    } else {
        return Err(McpError {
            code: ErrorCode::InvalidParams,
            message: "Missing params".to_string(),
            data: None,
        });
    };
    
    let response = resource_handler::handle_read_resource(&request)?;
    
    serde_json::to_value(response).map_err(|e| McpError {
        code: ErrorCode::InternalError,
        message: format!("Serialization error: {}", e),
        data: None,
    })
}

// Prompt handlers - may not be available

fn handle_list_prompts(params: Option<Value>) -> Result<Value, McpError> {
    let request: ListPromptsRequest = if let Some(p) = params {
        serde_json::from_value(p).map_err(|e| McpError {
            code: ErrorCode::InvalidParams,
            message: format!("Invalid params: {}", e),
            data: None,
        })?
    } else {
        ListPromptsRequest {
            cursor: None,
            progress_token: None,
            meta: None,
        }
    };
    
    let response = prompt_handler::handle_list_prompts(&request)?;
    
    serde_json::to_value(response).map_err(|e| McpError {
        code: ErrorCode::InternalError,
        message: format!("Serialization error: {}", e),
        data: None,
    })
}

fn handle_get_prompt(params: Option<Value>) -> Result<Value, McpError> {
    let request: GetPromptRequest = if let Some(p) = params {
        serde_json::from_value(p).map_err(|e| McpError {
            code: ErrorCode::InvalidParams,
            message: format!("Invalid params: {}", e),
            data: None,
        })?
    } else {
        return Err(McpError {
            code: ErrorCode::InvalidParams,
            message: "Missing params".to_string(),
            data: None,
        });
    };
    
    let response = prompt_handler::handle_get_prompt(&request)?;
    
    serde_json::to_value(response).map_err(|e| McpError {
        code: ErrorCode::InternalError,
        message: format!("Serialization error: {}", e),
        data: None,
    })
}