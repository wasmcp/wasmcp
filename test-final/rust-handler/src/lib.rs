mod bindings;

use bindings::exports::wasmcp::mcp::handler::Guest;
use bindings::wasmcp::mcp::types::{
    Error, PromptDescriptor, PromptResult, ResourceInfo, ResourceResult, Tool, ToolResult,
};

struct Component;

impl Guest for Component {
    fn list_tools() -> Vec<Tool> {
        vec![
            Tool {
                name: "echo".to_string(),
                description: "Echo a message back to the user".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Message to echo back"
                        }
                    },
                    "required": ["message"]
                })
                .to_string(),
            },
            Tool {
                name: "calculator".to_string(),
                description: "Perform basic arithmetic operations".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "operation": {
                            "type": "string",
                            "enum": ["add", "subtract", "multiply", "divide"],
                            "description": "Operation to perform"
                        },
                        "a": {
                            "type": "number",
                            "description": "First operand"
                        },
                        "b": {
                            "type": "number",
                            "description": "Second operand"
                        }
                    },
                    "required": ["operation", "a", "b"]
                })
                .to_string(),
            },
        ]
    }

    fn call_tool(name: String, arguments: String) -> ToolResult {
        match name.as_str() {
            "echo" => {
                match serde_json::from_str::<serde_json::Value>(&arguments) {
                    Ok(args) => {
                        if let Some(message) = args["message"].as_str() {
                            ToolResult::Text(format!("Echo: {}", message))
                        } else {
                            ToolResult::Error(Error {
                                code: -32602,
                                message: "Missing 'message' field".to_string(),
                                data: None,
                            })
                        }
                    }
                    Err(e) => ToolResult::Error(Error {
                        code: -32700,
                        message: format!("Invalid JSON: {}", e),
                        data: None,
                    }),
                }
            }
            "calculator" => {
                match serde_json::from_str::<serde_json::Value>(&arguments) {
                    Ok(args) => {
                        let operation = args["operation"].as_str().unwrap_or("");
                        let a = args["a"].as_f64().unwrap_or(0.0);
                        let b = args["b"].as_f64().unwrap_or(0.0);
                        
                        let result = match operation {
                            "add" => a + b,
                            "subtract" => a - b,
                            "multiply" => a * b,
                            "divide" => {
                                if b != 0.0 {
                                    a / b
                                } else {
                                    return ToolResult::Error(Error {
                                        code: -32602,
                                        message: "Division by zero".to_string(),
                                        data: None,
                                    });
                                }
                            }
                            _ => {
                                return ToolResult::Error(Error {
                                    code: -32602,
                                    message: format!("Unknown operation: {}", operation),
                                    data: None,
                                });
                            }
                        };
                        
                        ToolResult::Text(format!("{} {} {} = {}", a, operation, b, result))
                    }
                    Err(e) => ToolResult::Error(Error {
                        code: -32700,
                        message: format!("Invalid JSON: {}", e),
                        data: None,
                    }),
                }
            }
            _ => ToolResult::Error(Error {
                code: -32601,
                message: format!("Unknown tool: {}", name),
                data: None,
            }),
        }
    }

    fn list_resources() -> Vec<ResourceInfo> {
        vec![ResourceInfo {
            uri: "mcp://system/info".to_string(),
            name: "System Information".to_string(),
            description: Some("Runtime and version information".to_string()),
            mime_type: Some("application/json".to_string()),
        }]
    }

    fn read_resource(uri: String) -> ResourceResult {
        match uri.as_str() {
            "mcp://system/info" => {
                let info = serde_json::json!({
                    "version": "0.1.0",
                    "runtime": "Rust/WASI",
                    "capabilities": ["synchronous_execution"]
                });
                ResourceResult::Contents(bindings::wasmcp::mcp::types::ResourceContents {
                    uri,
                    mime_type: Some("application/json".to_string()),
                    text: Some(info.to_string()),
                    blob: None,
                })
            }
            _ => ResourceResult::Error(Error {
                code: -32002,
                message: format!("Resource not found: {}", uri),
                data: None,
            }),
        }
    }

    fn list_prompts() -> Vec<PromptDescriptor> {
        vec![]
    }

    fn get_prompt(_name: String, _arguments: String) -> PromptResult {
        PromptResult::Error(Error {
            code: -32002,
            message: "No prompts available".to_string(),
            data: None,
        })
    }
}

bindings::export!(Component with_types_in bindings);