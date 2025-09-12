use std::sync::OnceLock;

use anyhow::Result;
#[cfg(feature = "tools")]
use rmcp::model::CallToolRequestParam;
use rmcp::model::InitializeRequestParam;
#[cfg(any(feature = "tools", feature = "resources", feature = "prompts"))]
use rmcp::model::PaginatedRequestParam;
use serde_json::{Value, json};
use spin_sdk::http::{IntoResponse, Request, Response};

// cargo-component will generate bindings automatically
#[allow(warnings)]
mod bindings;

// Always import authorization types (no longer feature-gated)
use bindings::wasmcp::mcp::authorization;
use bindings::wasmcp::mcp::authorization_types::{AuthContext, ProviderAuthConfig};
use bindings::wasmcp::mcp::mcp_types::{ErrorCode, McpError};

mod adapter;
use adapter::WitMcpAdapter;

// Internal auth modules (always included now)
mod auth;
mod auth_types;
mod discovery;
mod jwt;
mod policy;

use auth_types::{AuthRequest, AuthResponse};

/// Static storage for auth configuration
static AUTH_CONFIG: OnceLock<Option<ProviderAuthConfig>> = OnceLock::new();

/// Get the cached auth configuration
/// Returns None if provider doesn't require auth
fn get_auth_config() -> &'static Option<ProviderAuthConfig> {
    AUTH_CONFIG.get_or_init(|| {
        // Provider can return None to disable auth
        authorization::get_auth_config()
    })
}

/// Check if auth is enabled (provider returned Some config)
fn is_auth_enabled() -> bool {
    get_auth_config().is_some()
}

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
}

#[spin_sdk::http_component]
async fn handle_request(req: Request) -> Result<impl IntoResponse> {
    let uri = req.uri();
    // Extract just the path from the URI
    let path = if uri.contains("://") {
        uri.split_once("://")
            .and_then(|(_, rest)| rest.split_once('/'))
            .map(|(_, path)| format!("/{path}"))
            .unwrap_or_else(|| "/".to_string())
    } else {
        uri.to_string()
    };
    eprintln!("DEBUG: Received request for path: {path} (from URI: {uri})");

    // Handle OAuth discovery endpoints only if auth is enabled
    if is_auth_enabled() {
        eprintln!("DEBUG: Auth is enabled, checking discovery endpoints");
        if path == "/.well-known/oauth-protected-resource" {
            eprintln!("DEBUG: Handling resource metadata endpoint");
            return handle_resource_metadata(req.uri());
        }
        if path == "/.well-known/oauth-authorization-server" {
            eprintln!("DEBUG: Handling server metadata endpoint");
            // For compatibility with clients that don't support resource metadata,
            // we provide the authorization server metadata directly
            return handle_server_metadata();
        }
    }

    // Apply authorization only if auth is enabled
    let auth_context = if is_auth_enabled() {
        match authorize_request(&req).await {
            Ok(context) => Some(context),
            Err(auth_error) => return Ok(create_auth_error_response(auth_error)),
        }
    } else {
        None
    };

    // Handle standard JSON-RPC endpoint
    let body = req.body();
    let request_str = std::str::from_utf8(body)?;

    // Create the MCP server
    let server = McpServer::new();

    // Parse JSON-RPC request
    let json_request: Value = serde_json::from_str(request_str)?;

    // Extract method and params
    let method = json_request["method"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing method"))?;
    let params = json_request.get("params").cloned();
    let id = json_request.get("id").cloned();

    // Route to appropriate handler
    match route_method(&server, method, params, auth_context).await {
        Ok(result) => {
            let response = if let Some(id) = id {
                json!({
                    "jsonrpc": "2.0",
                    "result": result,
                    "id": id
                })
            } else {
                // Notification (no id) - no response expected
                return Ok(Response::builder()
                    .status(204)
                    .header("content-type", "application/json")
                    .body(())
                    .build());
            };

            Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(response.to_string())
                .build())
        }
        Err(error) => {
            let error_response = json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": error.code.to_code(),
                    "message": error.message,
                    "data": error.data
                },
                "id": id
            });

            Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(error_response.to_string())
                .build())
        }
    }
}

/// Route method calls to appropriate handlers
/// Auth context is None if auth is disabled by provider
async fn route_method(
    server: &McpServer,
    method: &str,
    params: Option<Value>,
    auth_context: Option<AuthContext>,
) -> Result<Value, McpError> {
    match method {
        // Core lifecycle methods - always available
        "initialize" => {
            // Convert JSON-RPC params to WIT request
            let wit_request = if let Some(p) = params {
                // Parse the incoming params
                let params: InitializeRequestParam =
                    serde_json::from_value(p).map_err(|e| McpError {
                        code: ErrorCode::InvalidParams,
                        message: format!("Invalid params: {e}"),
                        data: None,
                    })?;

                // Use proper conversion from adapter
                server.adapter.convert_initialize_request(params)
                    .map_err(|e| McpError {
                        code: ErrorCode::InvalidParams,
                        message: format!("Invalid initialize params: {e}"),
                        data: None,
                    })?
            } else {
                // Default request when no params provided
                bindings::wasmcp::mcp::lifecycle_types::InitializeRequest {
                    protocol_version: bindings::wasmcp::mcp::lifecycle_types::ProtocolVersion::V20250618,
                    capabilities: bindings::wasmcp::mcp::lifecycle_types::ClientCapabilities {
                        experimental: None,
                        roots: None,
                        sampling: None,
                        elicitation: None,
                    },
                    client_info: bindings::wasmcp::mcp::lifecycle_types::ImplementationInfo {
                        name: "unknown".to_string(),
                        version: "0.0.0".to_string(),
                        title: None,
                    },
                    meta: None,
                }
            };

            // Call the provider's initialize handler
            let response = bindings::wasmcp::mcp::lifecycle::initialize(&wit_request)?;

            // Convert WIT response back to rmcp/JSON format
            let result = server
                .adapter
                .convert_initialize_to_rmcp(response)
                .map_err(|e| McpError {
                    code: ErrorCode::InternalError,
                    message: e.to_string(),
                    data: None,
                })?;
            Ok(serde_json::to_value(result).unwrap())
        }

        "notifications/initialized" => {
            // Client notification that it's ready for normal operations
            // Call the provider's client-initialized handler
            bindings::wasmcp::mcp::lifecycle::client_initialized()?;
            Ok(Value::Null)
        }

        "ping" => {
            // MCP spec doesn't define ping in lifecycle, just return success
            Ok(json!({}))
        }

        "shutdown" => {
            // Call the provider's shutdown handler
            bindings::wasmcp::mcp::lifecycle::shutdown()?;
            Ok(json!({}))
        }

        // Tools methods - only if feature enabled
        #[cfg(feature = "tools")]
        "tools/list" => {
            let _params: Option<PaginatedRequestParam> = params
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e| McpError {
                    code: ErrorCode::InvalidParams,
                    message: format!("Invalid params: {e}"),
                    data: None,
                })?;

            let request = bindings::wasmcp::mcp::tools_types::ListToolsRequest {
                cursor: None,
                progress_token: None,
                meta: None,
            };

            let response = bindings::wasmcp::mcp::tools::list_tools(&request)?;
            let result = server
                .adapter
                .convert_list_tools_to_rmcp(response)
                .map_err(|e| McpError {
                    code: ErrorCode::InternalError,
                    message: e.to_string(),
                    data: None,
                })?;
            Ok(serde_json::to_value(result).unwrap())
        }

        #[cfg(feature = "tools")]
        "tools/call" => {
            let params: CallToolRequestParam = params
                .ok_or_else(|| McpError {
                    code: ErrorCode::InvalidParams,
                    message: "Missing params".to_string(),
                    data: None,
                })
                .and_then(|p| {
                    serde_json::from_value(p).map_err(|e| McpError {
                        code: ErrorCode::InvalidParams,
                        message: format!("Invalid params: {e}"),
                        data: None,
                    })
                })?;

            // Convert request to WIT types
            let request = server
                .adapter
                .convert_call_tool_request(&params.name, params.arguments);

            // Call WIT binding directly with auth context if available (None if auth disabled)
            let response = bindings::wasmcp::mcp::tools::call_tool(&request, auth_context.as_ref())
                .map_err(|e| McpError {
                    code: ErrorCode::InternalError,
                    message: e.message,
                    data: None,
                })?;

            // Convert WIT response to rmcp result
            let result = server
                .adapter
                .convert_call_tool_to_rmcp(response)
                .map_err(|e| McpError {
                    code: ErrorCode::InternalError,
                    message: e.to_string(),
                    data: None,
                })?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // Resources methods - only if feature enabled
        #[cfg(feature = "resources")]
        "resources/list" => {
            let _params: Option<PaginatedRequestParam> = params
                .map(|p| serde_json::from_value(p))
                .transpose()
                .map_err(|e| McpError {
                    code: ErrorCode::InvalidParams,
                    message: format!("Invalid params: {}", e),
                    data: None,
                })?;

            let request = bindings::wasmcp::mcp::resources_types::ListResourcesRequest {
                cursor: None,
                progress_token: None,
                meta: None,
            };

            let response = bindings::wasmcp::mcp::resources::list_resources(&request)?;
            let result = server
                .adapter
                .convert_list_resources_to_rmcp(response)
                .map_err(|e| McpError {
                    code: ErrorCode::InternalError,
                    message: e.to_string(),
                    data: None,
                })?;
            Ok(serde_json::to_value(result).unwrap())
        }

        #[cfg(feature = "resources")]
        "resources/read" => {
            let params = params.ok_or_else(|| McpError {
                code: ErrorCode::InvalidParams,
                message: "Missing params".to_string(),
                data: None,
            })?;

            let uri = params["uri"].as_str().ok_or_else(|| McpError {
                code: ErrorCode::InvalidParams,
                message: "Missing uri parameter".to_string(),
                data: None,
            })?;

            let request = bindings::wasmcp::mcp::resources_types::ReadResourceRequest {
                uri: uri.to_string(),
                progress_token: None,
                meta: None,
            };

            let response = bindings::wasmcp::mcp::resources::read_resource(&request)?;
            let result = server
                .adapter
                .convert_read_resource_to_rmcp(response)
                .map_err(|e| McpError {
                    code: ErrorCode::InternalError,
                    message: e.to_string(),
                    data: None,
                })?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // Prompts methods - only if feature enabled
        #[cfg(feature = "prompts")]
        "prompts/list" => {
            let _params: Option<PaginatedRequestParam> = params
                .map(|p| serde_json::from_value(p))
                .transpose()
                .map_err(|e| McpError {
                    code: ErrorCode::InvalidParams,
                    message: format!("Invalid params: {}", e),
                    data: None,
                })?;

            let request = bindings::wasmcp::mcp::prompts_types::ListPromptsRequest {
                cursor: None,
                progress_token: None,
                meta: None,
            };

            let response = bindings::wasmcp::mcp::prompts::list_prompts(&request)?;
            let result = server
                .adapter
                .convert_list_prompts_to_rmcp(response)
                .map_err(|e| McpError {
                    code: ErrorCode::InternalError,
                    message: e.to_string(),
                    data: None,
                })?;
            Ok(serde_json::to_value(result).unwrap())
        }

        #[cfg(feature = "prompts")]
        "prompts/get" => {
            let params = params.ok_or_else(|| McpError {
                code: ErrorCode::InvalidParams,
                message: "Missing params".to_string(),
                data: None,
            })?;

            let name = params["name"].as_str().ok_or_else(|| McpError {
                code: ErrorCode::InvalidParams,
                message: "Missing name parameter".to_string(),
                data: None,
            })?;

            let arguments = params
                .get("arguments")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .map(|(k, v)| (k.clone(), v.to_string()))
                        .collect()
                });

            let request = bindings::wasmcp::mcp::prompts_types::GetPromptRequest {
                name: name.to_string(),
                arguments,
                progress_token: None,
                meta: None,
            };

            let response = bindings::wasmcp::mcp::prompts::get_prompt(&request)?;
            let result = server
                .adapter
                .convert_get_prompt_to_rmcp(response)
                .map_err(|e| McpError {
                    code: ErrorCode::InternalError,
                    message: e.to_string(),
                    data: None,
                })?;
            Ok(serde_json::to_value(result).unwrap())
        }

        // Completion methods - only if feature enabled
        #[cfg(feature = "completion")]
        "completion/complete" => {
            // TODO: Implement completion when WIT interface is defined
            Err(McpError {
                code: ErrorCode::InternalError,
                message: "Completion not yet implemented".to_string(),
                data: None,
            })
        }

        _ => Err(McpError {
            code: ErrorCode::MethodNotFound,
            message: format!("Method '{method}' not found"),
            data: None,
        }),
    }
}

/// Authorize request using the authorization component
async fn authorize_request(req: &Request) -> Result<AuthContext, McpError> {
    // Extract bearer token from Authorization header
    let token = req
        .headers()
        .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
        .and_then(|(_, value)| value.as_str())
        .and_then(|auth| auth.strip_prefix("Bearer "))
        .ok_or_else(|| McpError {
            code: ErrorCode::Unauthorized,
            message: "Missing or invalid Authorization header".to_string(),
            data: None,
        })?;

    // Collect request headers
    let headers: Vec<(String, String)> = req
        .headers()
        .map(|(name, value)| (name.to_string(), value.as_str().unwrap_or("").to_string()))
        .collect();

    // Get auth configuration - we know it's Some because authorize_request is only called when auth is enabled
    let provider_config = get_auth_config()
        .as_ref()
        .expect("authorize_request called but auth config is None");

    // Validate that the provider gave us valid auth config
    if provider_config.expected_issuer.is_empty() {
        return Err(McpError {
            code: ErrorCode::InternalError,
            message: "Provider returned invalid auth config: expected_issuer cannot be empty".to_string(),
            data: None,
        });
    }
    if provider_config.expected_audiences.is_empty() {
        return Err(McpError {
            code: ErrorCode::InternalError,
            message: "Provider returned invalid auth config: expected_audiences cannot be empty".to_string(),
            data: None,
        });
    }
    if provider_config.jwks_uri.is_empty() {
        return Err(McpError {
            code: ErrorCode::InternalError,
            message: "Provider returned invalid auth config: jwks_uri cannot be empty".to_string(),
            data: None,
        });
    }

    // Build authorization request with provider's required configuration
    let auth_request = AuthRequest {
        token: token.to_string(),
        method: req.method().to_string(),
        path: req.uri().to_string(),
        headers,
        body: Some(req.body().to_vec()),
        expected_issuer: provider_config.expected_issuer.clone(),
        expected_audiences: provider_config.expected_audiences.clone(),
        expected_subject: provider_config.expected_subject.clone(),
        jwks_uri: provider_config.jwks_uri.clone(),
        policy: provider_config.policy.clone(),
        policy_data: provider_config.policy_data.clone(),
        pass_jwt: provider_config.pass_jwt,
    };

    // Call the internal authorization function
    match auth::authorize(auth_request) {
        AuthResponse::Authorized(context) => Ok(context),
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

fn handle_resource_metadata(request_uri: &str) -> Result<Response> {
    if !is_auth_enabled() {
        return Err(anyhow::anyhow!("Resource metadata endpoint requires auth to be enabled"));
    }
    
    let provider_config = get_auth_config()
        .as_ref()
        .expect("handle_resource_metadata called but auth config is None");
    
    // Build the server URL from the request
    let server_url = if request_uri.contains("://") {
        request_uri
            .split_once("/.well-known")
            .map(|(base, _)| base.to_string())
            .unwrap_or_else(|| "http://localhost:8080".to_string())
    } else {
        "http://localhost:8080".to_string()
    };

    let metadata = discovery::get_resource_metadata(provider_config, &server_url);
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
        .body(json.to_string())
        .build())
}

fn handle_server_metadata() -> Result<Response> {
    if !is_auth_enabled() {
        return Err(anyhow::anyhow!("Server metadata endpoint requires auth to be enabled"));
    }
    
    let provider_config = get_auth_config()
        .as_ref()
        .expect("handle_server_metadata called but auth config is None");
        
    let metadata = discovery::get_server_metadata(provider_config);
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
        .body(json.to_string())
        .build())
}

fn create_auth_error_response(error: McpError) -> Response {
    let status = if error.code.to_code() == -32005 {
        401
    } else {
        403
    };
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "error": {
            "code": error.code.to_code(),
            "message": error.message,
            "data": error.data
        }
    });

    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(body.to_string())
        .build()
}

// ErrorCode extension trait
trait ErrorCodeExt {
    fn to_code(&self) -> i32;
}

impl ErrorCodeExt for ErrorCode {
    fn to_code(&self) -> i32 {
        match self {
            ErrorCode::ParseError => -32700,
            ErrorCode::InvalidRequest => -32600,
            ErrorCode::MethodNotFound => -32601,
            ErrorCode::InvalidParams => -32602,
            ErrorCode::InternalError => -32603,
            ErrorCode::ResourceNotFound => -32001,
            ErrorCode::ToolNotFound => -32002,
            ErrorCode::PromptNotFound => -32003,
            ErrorCode::Unauthorized => -32005,
            ErrorCode::RateLimited => -32006,
            ErrorCode::Timeout => -32007,
            ErrorCode::Cancelled => -32008,
            ErrorCode::CustomCode(code) => *code,
        }
    }
}