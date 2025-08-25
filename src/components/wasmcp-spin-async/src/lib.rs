use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use spin_sdk::http::{IntoResponse, Request, Response};
use tokio::sync::oneshot;

// These bindings would be generated from the async WIT file
mod bindings {
    // Placeholder for generated bindings
    pub mod wasmcp {
        pub mod mcp {
            pub mod types {
                pub struct Tool {
                    pub name: String,
                    pub description: String,
                    pub input_schema: String,
                }
                
                pub enum ToolResult {
                    Text(String),
                    Error { code: i32, message: String, data: Option<String> },
                }
                
                // The outparam resource
                pub struct ToolResponseOutparam {
                    sender: tokio::sync::oneshot::Sender<ToolResult>,
                }
                
                impl ToolResponseOutparam {
                    pub fn new(sender: tokio::sync::oneshot::Sender<ToolResult>) -> Self {
                        Self { sender }
                    }
                    
                    pub fn set(self, result: ToolResult) {
                        let _ = self.sender.send(result);
                    }
                }
            }
            
            pub mod handler {
                use super::types::*;
                
                // These would be the imported functions from the handler component
                pub fn list_tools() -> Vec<Tool> {
                    // This would call the actual handler
                    vec![]
                }
                
                pub fn call_tool(name: String, arguments: String, response_out: ToolResponseOutparam) {
                    // This would call the actual handler
                    // The handler will call response_out.set() when ready
                }
            }
        }
    }
}

use bindings::wasmcp::mcp::{handler, types::*};

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
async fn handle_request(req: Request) -> Result<impl IntoResponse> {
    let body = req.body();
    let request: JsonRpcRequest = serde_json::from_slice(body)?;

    let response = match request.method.as_str() {
        "tools/list" => {
            let tools = handler::list_tools();
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

                // Create a oneshot channel for the response
                let (tx, rx) = oneshot::channel();
                let response_out = ToolResponseOutparam::new(tx);
                
                // Call the handler with the outparam
                handler::call_tool(name.to_string(), arguments, response_out);
                
                // Wait for the response
                match rx.await {
                    Ok(ToolResult::Text(text)) => JsonRpcResponse {
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
                    Ok(ToolResult::Error { code, message, data }) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: JsonRpcResult::Error {
                            error: JsonRpcError {
                                code,
                                message,
                                data: data.map(Value::String),
                            },
                        },
                        id: request.id,
                    },
                    Err(_) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: JsonRpcResult::Error {
                            error: JsonRpcError {
                                code: -32603,
                                message: "Handler failed to respond".to_string(),
                                data: None,
                            },
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

    let body = serde_json::to_string(&response)?;
    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(body)
        .build())
}