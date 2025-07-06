use spin_sdk::http::{IntoResponse, Request, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// Generate bindings for the MCP dependency
wit_bindgen::generate!({
    world: "mcp-http-api",
    path: "./wit"
});

use component::mcp::handler::{
    list_tools, call_tool, list_resources, read_resource, list_prompts, get_prompt,
    ToolResult
};

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

#[spin_sdk::http_component]
fn handle_request(req: Request) -> anyhow::Result<impl IntoResponse> {
    let body = req.body();
    let request: JsonRpcRequest = serde_json::from_slice(body)?;
    
    let response = match request.method.as_str() {
        "initialize" => {
            JsonRpcResponse {
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
                            "name": "spin-mcp-gateway",
                            "version": "0.1.0"
                        }
                    }),
                },
                id: request.id,
            }
        }
        
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
            if let Some(params) = request.params {
                let name = params["name"].as_str().unwrap_or("");
                let arguments = params["arguments"].to_string();
                
                let result = call_tool(&name.to_string(), &arguments);
                
                match result {
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
                    ToolResult::Error(error) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: JsonRpcResult::Error { 
                            error: JsonRpcError {
                                code: error.code,
                                message: error.message,
                                data: error.data.map(|d| Value::String(d)),
                            }
                        },
                        id: request.id,
                    },
                }
            } else {
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: JsonRpcResult::Error {
                        error: JsonRpcError {
                            code: -32602,
                            message: "Invalid params".to_string(),
                            data: None,
                        },
                    },
                    id: request.id,
                }
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
            if let Some(params) = request.params {
                let uri = params["uri"].as_str().unwrap_or("");
                
                match read_resource(&uri.to_string()) {
                    Ok(contents) => {
                        let contents_json = serde_json::json!([{
                            "uri": contents.uri,
                            "mimeType": contents.mime_type,
                            "text": contents.text,
                            "blob": contents.blob.map(|b| base64_encode(b))
                        }]);
                        
                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: JsonRpcResult::Result {
                                result: serde_json::json!({ "contents": contents_json }),
                            },
                            id: request.id,
                        }
                    }
                    Err(error) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: JsonRpcResult::Error { 
                            error: JsonRpcError {
                                code: error.code,
                                message: error.message,
                                data: error.data.map(|d| Value::String(d)),
                            }
                        },
                        id: request.id,
                    },
                }
            } else {
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: JsonRpcResult::Error {
                        error: JsonRpcError {
                            code: -32602,
                            message: "Invalid params".to_string(),
                            data: None,
                        },
                    },
                    id: request.id,
                }
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
            if let Some(params) = request.params {
                let name = params["name"].as_str().unwrap_or("");
                let arguments = params.get("arguments").map(|a| a.to_string()).unwrap_or_else(|| "{}".to_string());
                
                match get_prompt(&name.to_string(), &arguments) {
                    Ok(messages) => {
                        let messages_json: Vec<Value> = messages.into_iter().map(|m| {
                            serde_json::json!({
                                "role": m.role,
                                "content": m.content
                            })
                        }).collect();
                        
                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: JsonRpcResult::Result {
                                result: serde_json::json!({ "messages": messages_json }),
                            },
                            id: request.id,
                        }
                    }
                    Err(error) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: JsonRpcResult::Error { 
                            error: JsonRpcError {
                                code: error.code,
                                message: error.message,
                                data: error.data.map(|d| Value::String(d)),
                            }
                        },
                        id: request.id,
                    },
                }
            } else {
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: JsonRpcResult::Error {
                        error: JsonRpcError {
                            code: -32602,
                            message: "Invalid params".to_string(),
                            data: None,
                        },
                    },
                    id: request.id,
                }
            }
        }
        
        "ping" => {
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: JsonRpcResult::Result {
                    result: serde_json::json!({}),
                },
                id: request.id,
            }
        }
        
        _ => {
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: JsonRpcResult::Error {
                    error: JsonRpcError {
                        code: -32601,
                        message: "Method not found".to_string(),
                        data: None,
                    },
                },
                id: request.id,
            }
        }
    };
    
    let body = serde_json::to_string(&response)?;
    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(body)
        .build())
}

fn base64_encode(data: Vec<u8>) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}