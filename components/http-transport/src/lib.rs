use spin_sdk::http::{IntoResponse, Request, Response};
use anyhow::Result;
use serde_json::{json, Value};

// Use rmcp types for protocol compliance
use rmcp::model::{
    ServerInfo,
    InitializeRequestParam, ServerCapabilities as RmcpServerCapabilities,
    Implementation, ProtocolVersion,
};

#[cfg(feature = "tools")]
use rmcp::model::CallToolRequestParam;

#[cfg(any(feature = "tools", feature = "resources", feature = "prompts"))]
use rmcp::model::PaginatedRequestParam;

// cargo-component will generate bindings automatically
#[allow(warnings)]
mod bindings;

// Always import types and session for core functionality
use bindings::fastertools::mcp::{
    types::{McpError, ErrorCode},
};

// Authorization imports when feature is enabled
#[cfg(feature = "auth")]
use bindings::fastertools::mcp::{
    authorization::{AuthRequest, AuthResponse},
    oauth_discovery,
};

mod adapter;
use adapter::WitMcpAdapter;

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
    
    /// Get server info using rmcp types for client compatibility
    fn get_server_info(&self) -> Result<ServerInfo> {
        Ok(ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: RmcpServerCapabilities {
                #[cfg(feature = "tools")]
                tools: Some(rmcp::model::ToolsCapability {
                    list_changed: Some(false),
                }),
                #[cfg(not(feature = "tools"))]
                tools: None,
                
                #[cfg(feature = "resources")]
                resources: Some(rmcp::model::ResourcesCapability {
                    #[cfg(feature = "sse")]
                    subscribe: Some(true),
                    #[cfg(not(feature = "sse"))]
                    subscribe: None,
                    list_changed: Some(false),
                }),
                #[cfg(not(feature = "resources"))]
                resources: None,
                
                #[cfg(feature = "prompts")]
                prompts: Some(rmcp::model::PromptsCapability {
                    list_changed: Some(false),
                }),
                #[cfg(not(feature = "prompts"))]
                prompts: None,
                
                ..Default::default()
            },
            server_info: Implementation {
                name: "wasmcp-server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: None,
        })
    }
}

#[spin_sdk::http_component]
async fn handle_request(req: Request) -> Result<impl IntoResponse> {
    let uri = req.uri();
    // Extract just the path from the URI
    let path = if uri.contains("://") {
        uri.split_once("://")
            .and_then(|(_, rest)| rest.split_once('/'))
            .map(|(_, path)| format!("/{}", path))
            .unwrap_or_else(|| "/".to_string())
    } else {
        uri.to_string()
    };
    eprintln!("DEBUG: Received request for path: {} (from URI: {})", path, uri);
    
    // Handle OAuth discovery endpoints when auth is enabled
    #[cfg(feature = "auth")]
    {
        eprintln!("DEBUG: Auth feature is enabled, checking discovery endpoints");
        if path == "/.well-known/oauth-protected-resource" {
            eprintln!("DEBUG: Handling resource metadata endpoint");
            return handle_resource_metadata();
        }
        if path == "/.well-known/oauth-authorization-server" {
            eprintln!("DEBUG: Handling server metadata endpoint");
            return handle_server_metadata();
        }
    }
    
    // Handle SSE endpoint if enabled
    #[cfg(feature = "sse")]
    if path == "/mcp/sse" {
        return handle_sse_request(req).await;
    }
    
    // Apply authorization if enabled
    #[cfg(feature = "auth")]
    {
        if let Err(auth_error) = authorize_request(&req).await {
            return Ok(create_auth_error_response(auth_error));
        }
    }
    
    // Handle standard JSON-RPC endpoint
    let body = req.body();
    let request_str = std::str::from_utf8(body)?;
    
    // Create the MCP server
    let server = McpServer::new();
    
    // Parse JSON-RPC request
    let json_request: Value = serde_json::from_str(request_str)?;
    
    // Extract method and handle accordingly
    let method = json_request.get("method")
        .and_then(|m| m.as_str())
        .unwrap_or("");
    
    let id = json_request.get("id").cloned();
    let params = json_request.get("params").cloned();
    
    // Route to appropriate handler based on enabled features
    let result = route_method(&server, method, params).await;
    
    // Handle notifications (no id)
    if id.is_none() && result.is_ok() {
        return Ok(Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .body("")
            .build());
    }
    
    // Build JSON-RPC response
    let response_json = match result {
        Ok(result) => {
            json!({
                "jsonrpc": "2.0",
                "result": result,
                "id": id
            })
        },
        Err(e) => {
            json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": e.code.into_code(),
                    "message": e.message,
                    "data": e.data
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

/// Route method calls based on enabled features
async fn route_method(server: &McpServer, method: &str, params: Option<Value>) -> Result<Value, McpError> {
    match method {
        // Core methods - always available
        "initialize" => {
            // Use rmcp type for deserialization
            let _params: InitializeRequestParam = if let Some(p) = params {
                serde_json::from_value(p).map_err(|e| McpError {
                    code: ErrorCode::InvalidParams,
                    message: format!("Invalid params: {}", e),
                    data: None,
                })?
            } else {
                InitializeRequestParam::default()
            };
            
            let server_info = server.get_server_info().map_err(|e| McpError {
                code: ErrorCode::InternalError,
                message: e.to_string(),
                data: None,
            })?;
            Ok(serde_json::to_value(server_info).unwrap())
        },
        
        "initialized" | "notifications/initialized" => Ok(Value::Null),
        "ping" => Ok(json!({})),
        "shutdown" => Ok(json!({})),
        
        // Tools methods - only if feature enabled
        #[cfg(feature = "tools")]
        "tools/list" => {
                let _params: Option<PaginatedRequestParam> = params
                    .map(|p| serde_json::from_value(p))
                    .transpose()
                    .map_err(|e| McpError {
                        code: ErrorCode::InvalidParams,
                        message: format!("Invalid params: {}", e),
                        data: None,
                    })?;
                
                let request = bindings::fastertools::mcp::tools::ListToolsRequest {
                    cursor: None,
                    progress_token: None,
                    meta: None,
                };
                
                let response = bindings::fastertools::mcp::tools_capabilities::handle_list_tools(&request)?;
                let result = server.adapter.convert_list_tools_to_rmcp(response).map_err(|e| McpError {
                    code: ErrorCode::InternalError,
                    message: e.to_string(),
                    data: None,
                })?;
                Ok(serde_json::to_value(result).unwrap())
        },
        
        #[cfg(feature = "tools")]
        "tools/call" => {
                let params: CallToolRequestParam = serde_json::from_value(params.unwrap_or_default()).map_err(|e| McpError {
                    code: ErrorCode::InvalidParams,
                    message: format!("Invalid params: {}", e),
                    data: None,
                })?;
                let result = server.adapter.call_tool(&params.name, params.arguments).await.map_err(|e| McpError {
                    code: ErrorCode::InternalError,
                    message: e.to_string(),
                    data: None,
                })?;
                Ok(serde_json::to_value(result).unwrap())
        },
        
        // Resources methods - only if feature enabled
        #[cfg(feature = "resources")]
        "resources/list" => {
            let request = if let Some(p) = params {
                serde_json::from_value(p).map_err(|e| McpError {
                    code: ErrorCode::InvalidParams,
                    message: format!("Invalid params: {}", e),
                    data: None,
                })?
            } else {
                bindings::fastertools::mcp::resources::ListResourcesRequest {
                    cursor: None,
                    progress_token: None,
                    meta: None,
                }
            };
            
            let response = bindings::fastertools::mcp::resources_capabilities::handle_list_resources(&request)?;
            let result = server.adapter.convert_list_resources_to_rmcp(response).map_err(|e| McpError {
                code: ErrorCode::InternalError,
                message: e.to_string(),
                data: None,
            })?;
            Ok(serde_json::to_value(result).unwrap())
        },
        
        #[cfg(feature = "resources")]
        "resources/read" => {
            let request: bindings::fastertools::mcp::resources::ReadResourceRequest = 
                serde_json::from_value(params.unwrap_or_default()).map_err(|e| McpError {
                    code: ErrorCode::InvalidParams,
                    message: format!("Invalid params: {}", e),
                    data: None,
                })?;
            
            let response = bindings::fastertools::mcp::resources_capabilities::handle_read_resource(&request)?;
            let result = server.adapter.convert_read_resource_to_rmcp(response).map_err(|e| McpError {
                code: ErrorCode::InternalError,
                message: e.to_string(),
                data: None,
            })?;
            Ok(serde_json::to_value(result).unwrap())
        },
        
        #[cfg(all(feature = "resources", feature = "sse"))]
        "resources/subscribe" => {
            let request: bindings::fastertools::mcp::resources::SubscribeRequest = 
                serde_json::from_value(params.unwrap_or_default()).map_err(|e| McpError {
                    code: ErrorCode::InvalidParams,
                    message: format!("Invalid params: {}", e),
                    data: None,
                })?;
            
            bindings::fastertools::mcp::resources_capabilities::handle_subscribe_resource(&request)?;
            Ok(json!({}))
        },
        
        // Prompts methods - only if feature enabled
        #[cfg(feature = "prompts")]
        "prompts/list" => {
            let request = if let Some(p) = params {
                serde_json::from_value(p).map_err(|e| McpError {
                    code: ErrorCode::InvalidParams,
                    message: format!("Invalid params: {}", e),
                    data: None,
                })?
            } else {
                bindings::fastertools::mcp::prompts::ListPromptsRequest {
                    cursor: None,
                    progress_token: None,
                    meta: None,
                }
            };
            
            let response = bindings::fastertools::mcp::prompts_capabilities::handle_list_prompts(&request)?;
            let result = server.adapter.convert_list_prompts_to_rmcp(response).map_err(|e| McpError {
                code: ErrorCode::InternalError,
                message: e.to_string(),
                data: None,
            })?;
            Ok(serde_json::to_value(result).unwrap())
        },
        
        #[cfg(feature = "prompts")]
        "prompts/get" => {
            let request: bindings::fastertools::mcp::prompts::GetPromptRequest = 
                serde_json::from_value(params.unwrap_or_default()).map_err(|e| McpError {
                    code: ErrorCode::InvalidParams,
                    message: format!("Invalid params: {}", e),
                    data: None,
                })?;
            
            let response = bindings::fastertools::mcp::prompts_capabilities::handle_get_prompt(&request)?;
            let result = server.adapter.convert_get_prompt_to_rmcp(response).map_err(|e| McpError {
                code: ErrorCode::InternalError,
                message: e.to_string(),
                data: None,
            })?;
            Ok(serde_json::to_value(result).unwrap())
        },
        
        // Default case - method not found or not enabled
        _ => {
            Err(McpError {
                code: ErrorCode::MethodNotFound,
                message: format!("Method '{}' not found or not enabled in this server variant", method),
                data: None,
            })
        }
    }
}

// SSE support if enabled
#[cfg(feature = "sse")]
async fn handle_sse_request(req: Request) -> Result<impl IntoResponse> {
    // SSE implementation would go here
    // For now, just return a placeholder
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body("data: SSE endpoint enabled\n\n")
        .build())
}

// Extension trait for ErrorCode
impl ErrorCode {
    fn into_code(&self) -> i32 {
        match self {
            ErrorCode::ParseError => -32700,
            ErrorCode::InvalidRequest => -32600,
            ErrorCode::MethodNotFound => -32601,
            ErrorCode::InvalidParams => -32602,
            ErrorCode::InternalError => -32603,
            ErrorCode::ResourceNotFound => -32002,
            ErrorCode::ToolNotFound => -32003,
            ErrorCode::PromptNotFound => -32004,
            ErrorCode::Unauthorized => -32005,
            ErrorCode::RateLimited => -32006,
            ErrorCode::Timeout => -32007,
            ErrorCode::Cancelled => -32008,
            ErrorCode::CustomCode(code) => *code,
        }
    }
}

// Authorization support functions
#[cfg(feature = "auth")]
async fn authorize_request(req: &Request) -> Result<(), McpError> {
    // Extract bearer token from Authorization header
    let token = req.headers()
        .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
        .and_then(|(_, value)| value.as_str())
        .and_then(|auth| auth.strip_prefix("Bearer "))
        .ok_or_else(|| McpError {
            code: ErrorCode::Unauthorized,
            message: "Missing or invalid Authorization header".to_string(),
            data: None,
        })?;
    
    // Collect request headers
    let headers: Vec<(String, String)> = req.headers()
        .map(|(name, value)| {
            (name.to_string(), value.as_str().unwrap_or("").to_string())
        })
        .collect();
    
    // Build authorization request
    let auth_request = AuthRequest {
        token: token.to_string(),
        method: req.method().to_string(),
        path: req.uri().to_string(),
        headers,
        body: Some(req.body().to_vec()),
        expected_issuer: std::env::var("MCP_EXPECTED_ISSUER").ok(),
        expected_audience: std::env::var("MCP_EXPECTED_AUDIENCE").ok(),
        jwks_uri: std::env::var("MCP_JWKS_URI").ok(),
    };
    
    // Call the authorization component
    match bindings::fastertools::mcp::authorization::authorize(&auth_request) {
        AuthResponse::Authorized(_context) => Ok(()),
        AuthResponse::Unauthorized(error) => Err(McpError {
            code: if error.status == 403 {
                ErrorCode::Unauthorized
            } else {
                ErrorCode::InvalidRequest
            },
            message: error.description,
            data: error.www_authenticate,
        }),
    }
}

#[cfg(feature = "auth")]
fn handle_resource_metadata() -> Result<Response> {
    let metadata = oauth_discovery::get_resource_metadata();
    let json = serde_json::json!({
        "resource": metadata.resource_url,
        "authorization_servers": metadata.authorization_servers,
        "scopes_supported": metadata.scopes_supported,
        "bearer_methods_supported": metadata.bearer_methods_supported,
        "resource_documentation": metadata.resource_documentation,
    });
    
    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .header("access-control-allow-origin", "*")
        .body(serde_json::to_string(&json)?)
        .build())
}

#[cfg(feature = "auth")]
fn handle_server_metadata() -> Result<Response> {
    let metadata = oauth_discovery::get_server_metadata();
    let json = serde_json::json!({
        "issuer": metadata.issuer,
        "authorization_endpoint": metadata.authorization_endpoint,
        "token_endpoint": metadata.token_endpoint,
        "jwks_uri": metadata.jwks_uri,
        "response_types_supported": metadata.response_types_supported,
        "grant_types_supported": metadata.grant_types_supported,
        "code_challenge_methods_supported": metadata.code_challenge_methods_supported,
        "scopes_supported": metadata.scopes_supported,
        "token_endpoint_auth_methods_supported": metadata.token_endpoint_auth_methods_supported,
        "service_documentation": metadata.service_documentation,
        "registration_endpoint": metadata.registration_endpoint,
    });
    
    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .header("access-control-allow-origin", "*")
        .body(serde_json::to_string(&json)?)
        .build())
}

#[cfg(feature = "auth")]
fn create_auth_error_response(error: McpError) -> Response {
    let status = if error.code.into_code() == -32005 { 401 } else { 403 };
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "error": {
            "code": error.code.into_code(),
            "message": error.message,
            "data": error.data
        }
    });
    
    let mut response = Response::builder();
    let mut builder = response
        .status(status as u16)
        .header("content-type", "application/json")
        .header("access-control-allow-origin", "*");
    
    // Add WWW-Authenticate header if provided
    if let Some(www_auth) = error.data {
        builder = builder.header("www-authenticate", www_auth);
    }
    
    builder.body(serde_json::to_string(&body).unwrap()).build()
}