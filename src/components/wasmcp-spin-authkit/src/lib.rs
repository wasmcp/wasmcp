use serde::{Deserialize, Serialize};
use serde_json::Value;
use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::variables;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

// cargo-component will generate bindings automatically
#[allow(warnings)]
mod bindings;

use bindings::wasmcp::mcp::handler::{
    call_tool, get_prompt, list_prompts, list_resources, list_tools, read_resource, ToolResult,
};

/// Configuration for AuthKit
#[derive(Debug, Deserialize)]
struct AuthKitConfig {
    issuer: String,
    jwks_uri: String,
    audience: Option<String>,
}

/// JWT Claims structure
#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    iss: String,
    aud: Option<String>,
    exp: i64,
    iat: i64,
    email: Option<String>,
    #[serde(flatten)]
    extra: Value,
}

/// JSON-RPC request structure
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Option<Value>,
}

/// JSON-RPC response structure
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(flatten)]
    result: JsonRpcResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum JsonRpcResult {
    Result { result: Value },
    Error { error: JsonRpcError },
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// Check if path is a metadata endpoint
fn is_metadata_path(path: &str) -> bool {
    matches!(path, 
        "/.well-known/oauth-protected-resource" |
        "/.well-known/oauth-authorization-server"
    )
}

/// Extract bearer token from authorization header
fn extract_bearer_token(auth_header: &str) -> Option<&str> {
    auth_header
        .strip_prefix("Bearer ")
        .map(|s| s.trim())
}

/// Verify JWT token (simplified - doesn't check signature)
fn verify_token(token: &str, config: &AuthKitConfig) -> Result<Claims, String> {
    // Split token into parts
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid JWT format".to_string());
    }
    
    // Decode claims (base64url)
    let claims_json = URL_SAFE_NO_PAD.decode(parts[1])
        .map_err(|e| format!("Failed to decode token: {}", e))?;
    
    let claims: Claims = serde_json::from_slice(&claims_json)
        .map_err(|e| format!("Failed to parse claims: {}", e))?;
    
    // Verify issuer
    if claims.iss != config.issuer {
        return Err(format!("Invalid issuer: expected {}, got {}", config.issuer, claims.iss));
    }
    
    // Verify audience if configured
    if let Some(expected_aud) = &config.audience {
        if claims.aud.as_ref() != Some(expected_aud) {
            return Err("Invalid audience".to_string());
        }
    }
    
    // Check expiration (simplified - use actual time in production)
    let now = 1735689600; // Hardcoded for demo
    if claims.exp < now {
        return Err("Token expired".to_string());
    }
    
    Ok(claims)
}

/// Build authentication error response
fn auth_error_response(error: &str, host: Option<&str>) -> Response {
    let www_auth = if let Some(h) = host {
        format!(
            "Bearer error=\"unauthorized\", error_description=\"{}\", resource_metadata=\"https://{}/.well-known/oauth-protected-resource\"",
            error, h
        )
    } else {
        format!("Bearer error=\"unauthorized\", error_description=\"{}\"", error)
    };
    
    let body = serde_json::json!({
        "error": "unauthorized",
        "error_description": error
    });
    
    Response::builder()
        .status(401)
        .header("WWW-Authenticate", www_auth)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .build()
}

/// Handle metadata endpoints
fn handle_metadata(path: &str, config: &AuthKitConfig, host: Option<&str>) -> Option<Response> {
    match path {
        "/.well-known/oauth-protected-resource" => {
            // The resource should be this server's URL, not the AuthKit URL
            let resource_url = match host {
                Some(h) => format!("http://{}", h),
                None => "http://127.0.0.1:3000".to_string(),
            };
            
            let metadata = serde_json::json!({
                "resource": resource_url,
                "authorization_servers": [&config.issuer],
                "bearer_methods_supported": ["header"]
            });
            
            Some(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(metadata.to_string())
                .build())
        },
        _ => None
    }
}

#[spin_sdk::http_component]
async fn handle_request(req: Request) -> anyhow::Result<impl IntoResponse> {
    // Load AuthKit configuration
    let issuer = variables::get("authkit_issuer")
        .unwrap_or_else(|_| "https://example.authkit.app".to_string());
    let jwks_uri = variables::get("authkit_jwks_uri")
        .unwrap_or_else(|_| format!("{}/oauth2/jwks", issuer));
    let audience = variables::get("authkit_audience").ok();
    
    let config = AuthKitConfig {
        issuer,
        jwks_uri,
        audience,
    };
    
    let path = req.path();
    
    // Extract host for metadata
    let host = req.headers()
        .find(|(name, _)| name.eq_ignore_ascii_case("host"))
        .and_then(|(_, value)| value.as_str());
    
    // Handle metadata endpoints
    if is_metadata_path(path) {
        if let Some(response) = handle_metadata(path, &config, host) {
            return Ok(response);
        }
    }
    
    
    // Check authorization
    let auth_header = req.headers()
        .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
        .and_then(|(_, value)| value.as_str());
    
    let claims = if let Some(auth) = auth_header {
        if let Some(token) = extract_bearer_token(auth) {
            match verify_token(token, &config) {
                Ok(c) => Some(c),
                Err(e) => return Ok(auth_error_response(&e, host))
            }
        } else {
            return Ok(auth_error_response("Invalid authorization header format", host));
        }
    } else {
        return Ok(auth_error_response("Missing authorization header", host));
    };
    
    // Parse JSON-RPC request
    let body = req.body();
    let request: JsonRpcRequest = serde_json::from_slice(body)?;
    
    // Add user context to responses where applicable
    let user_info = claims.map(|c| serde_json::json!({
        "authenticated_user": c.sub,
        "email": c.email,
    }));
    
    // Handle MCP methods
    let response = match request.method.as_str() {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: JsonRpcResult::Result {
                result: serde_json::json!({
                    "protocolVersion": "2025-03-26",
                    "capabilities": {
                        "tools": {},
                        "resources": {},
                        "prompts": {}
                    },
                    "serverInfo": {
                        "name": "wasmcp-spin-authkit",
                        "version": "0.1.0",
                        "authInfo": user_info
                    }
                }),
            },
            id: request.id,
        },

        "initialized" => {
            // Notification, no response needed
            return Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body("".to_string())
                .build());
        }

        "tools/list" => {
            let tools = list_tools();
            let tools_json: Vec<Value> = tools.into_iter().map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": serde_json::from_str::<Value>(&t.input_schema).unwrap_or(Value::Null)
                })
            }).collect();

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: JsonRpcResult::Result {
                    result: serde_json::json!({ "tools": tools_json }),
                },
                id: request.id,
            }
        }

        "tools/call" => {
            let params = request.params.ok_or_else(|| anyhow::anyhow!("Missing params"))?;
            let name = params.get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;
            let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

            match call_tool(name, &arguments.to_string()) {
                ToolResult::Text(text) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: JsonRpcResult::Result {
                        result: serde_json::json!({
                            "content": [{
                                "type": "text",
                                "text": text
                            }]
                        }),
                    },
                    id: request.id,
                },
                ToolResult::Error(e) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: JsonRpcResult::Error {
                        error: JsonRpcError {
                            code: e.code,
                            message: e.message,
                            data: e.data.map(|d| serde_json::from_str(&d).unwrap_or(Value::Null)),
                        },
                    },
                    id: request.id,
                },
            }
        }

        "resources/list" => {
            let resources = list_resources();
            let resources_json: Vec<Value> = resources.into_iter().map(|r| {
                serde_json::json!({
                    "uri": r.uri,
                    "name": r.name,
                    "description": r.description,
                    "mimeType": r.mime_type
                })
            }).collect();

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: JsonRpcResult::Result {
                    result: serde_json::json!({ "resources": resources_json }),
                },
                id: request.id,
            }
        }

        "resources/read" => {
            let params = request.params.ok_or_else(|| anyhow::anyhow!("Missing params"))?;
            let uri = params.get("uri")
                .and_then(|u| u.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing resource URI"))?;

            match read_resource(uri) {
                Ok(contents) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: JsonRpcResult::Result {
                        result: serde_json::json!({
                            "contents": [{
                                "uri": contents.uri,
                                "mimeType": contents.mime_type,
                                "text": contents.text,
                                "blob": contents.blob.map(|b| base64::engine::general_purpose::STANDARD.encode(b))
                            }]
                        }),
                    },
                    id: request.id,
                },
                Err(e) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: JsonRpcResult::Error {
                        error: JsonRpcError {
                            code: e.code,
                            message: e.message,
                            data: e.data.map(|d| serde_json::from_str(&d).unwrap_or(Value::Null)),
                        },
                    },
                    id: request.id,
                },
            }
        }

        "prompts/list" => {
            let prompts = list_prompts();
            let prompts_json: Vec<Value> = prompts.into_iter().map(|p| {
                serde_json::json!({
                    "name": p.name,
                    "description": p.description,
                    "arguments": p.arguments.into_iter().map(|a| {
                        serde_json::json!({
                            "name": a.name,
                            "description": a.description,
                            "required": a.required
                        })
                    }).collect::<Vec<_>>()
                })
            }).collect();

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: JsonRpcResult::Result {
                    result: serde_json::json!({ "prompts": prompts_json }),
                },
                id: request.id,
            }
        }

        "prompts/get" => {
            let params = request.params.ok_or_else(|| anyhow::anyhow!("Missing params"))?;
            let name = params.get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing prompt name"))?;
            let arguments = params.get("arguments").cloned().unwrap_or(Value::Object(Default::default()));

            match get_prompt(name, &arguments.to_string()) {
                Ok(messages) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: JsonRpcResult::Result {
                        result: serde_json::json!({
                            "messages": messages.into_iter().map(|m| {
                                serde_json::json!({
                                    "role": m.role,
                                    "content": m.content
                                })
                            }).collect::<Vec<_>>()
                        }),
                    },
                    id: request.id,
                },
                Err(e) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: JsonRpcResult::Error {
                        error: JsonRpcError {
                            code: e.code,
                            message: e.message,
                            data: e.data.map(|d| serde_json::from_str(&d).unwrap_or(Value::Null)),
                        },
                    },
                    id: request.id,
                },
            }
        }

        _ => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: JsonRpcResult::Error {
                error: JsonRpcError {
                    code: -32601,
                    message: "Method not found".to_string(),
                    data: None,
                },
            },
            id: request.id,
        },
    };

    let response_body = serde_json::to_string(&response)?;
    
    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(response_body)
        .build())
}