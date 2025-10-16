//! Calculator Tools Capability Provider
//!
//! A clean tools capability that provides basic arithmetic operations.

#![allow(warnings)]

mod bindings {
    wit_bindgen::generate!({
        world: "calculator",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp::tools_capability::Guest;
use bindings::wasmcp::mcp::protocol::*;

struct Calculator;

impl Guest for Calculator {
    fn list_tools(
        request: ListToolsRequest,
        client: ClientContext,
    ) -> ListToolsResult {
        ListToolsResult {
            tools: vec![
                create_add_tool(),
                create_subtract_tool(),
                create_multiply_tool(),
                create_divide_tool(),
            ],
            next_cursor: None,
            meta: None,
        }
    }

    fn call_tool(
        request: CallToolRequest,
        client: ClientContext,
    ) -> Option<CallToolResult> {
        match request.name.as_str() {
            "add" => Some(execute_add(&request)),
            "subtract" => Some(execute_subtract(&request)),
            "multiply" => Some(execute_multiply(&request)),
            "divide" => Some(execute_divide(&request)),
            _ => None, // We don't handle this tool
        }
    }
}

// Tool definitions

fn create_add_tool() -> Tool {
    Tool {
        name: "add".to_string(),
        input_schema: r#"{
            "type": "object",
            "properties": {
                "a": {"type": "number", "description": "First number"},
                "b": {"type": "number", "description": "Second number"}
            },
            "required": ["a", "b"]
        }"#.to_string(),
        options: Some(ToolOptions {
            meta: None,
            annotations: None,
            description: Some("Add two numbers together".to_string()),
            output_schema: None,
            title: Some("Add".to_string()),
        }),
    }
}

fn create_subtract_tool() -> Tool {
    Tool {
        name: "subtract".to_string(),
        input_schema: r#"{
            "type": "object",
            "properties": {
                "a": {"type": "number", "description": "Number to subtract from"},
                "b": {"type": "number", "description": "Number to subtract"}
            },
            "required": ["a", "b"]
        }"#.to_string(),
        options: Some(ToolOptions {
            meta: None,
            annotations: None,
            description: Some("Subtract b from a".to_string()),
            output_schema: None,
            title: Some("Subtract".to_string()),
        }),
    }
}

fn create_multiply_tool() -> Tool {
    Tool {
        name: "multiply".to_string(),
        input_schema: r#"{
            "type": "object",
            "properties": {
                "a": {"type": "number", "description": "First number"},
                "b": {"type": "number", "description": "Second number"}
            },
            "required": ["a", "b"]
        }"#.to_string(),
        options: Some(ToolOptions {
            meta: None,
            annotations: None,
            description: Some("Multiply two numbers together".to_string()),
            output_schema: None,
            title: Some("Multiply".to_string()),
        }),
    }
}

fn create_divide_tool() -> Tool {
    Tool {
        name: "divide".to_string(),
        input_schema: r#"{
            "type": "object",
            "properties": {
                "a": {"type": "number", "description": "Numerator"},
                "b": {"type": "number", "description": "Denominator"}
            },
            "required": ["a", "b"]
        }"#.to_string(),
        options: Some(ToolOptions {
            meta: None,
            annotations: None,
            description: Some("Divide a by b".to_string()),
            output_schema: None,
            title: Some("Divide".to_string()),
        }),
    }
}

// Tool execution

fn execute_add(req: &CallToolRequest) -> CallToolResult {
    match parse_args(&req.arguments) {
        Ok((a, b)) => {
            let result = a + b;
            CallToolResult {
                content: vec![ContentBlock::Text(TextContent {
                    text: TextData::Text(result.to_string()),
                    options: None,
                })],
                is_error: None,
                meta: None,
                structured_content: None,
            }
        }
        Err(msg) => CallToolResult {
            content: vec![ContentBlock::Text(TextContent {
                text: TextData::Text(msg),
                options: None,
            })],
            is_error: Some(true),
            meta: None,
            structured_content: None,
        },
    }
}

fn execute_subtract(req: &CallToolRequest) -> CallToolResult {
    match parse_args(&req.arguments) {
        Ok((a, b)) => {
            let result = a - b;
            CallToolResult {
                content: vec![ContentBlock::Text(TextContent {
                    text: TextData::Text(result.to_string()),
                    options: None,
                })],
                is_error: None,
                meta: None,
                structured_content: None,
            }
        }
        Err(msg) => CallToolResult {
            content: vec![ContentBlock::Text(TextContent {
                text: TextData::Text(msg),
                options: None,
            })],
            is_error: Some(true),
            meta: None,
            structured_content: None,
        },
    }
}

fn execute_multiply(req: &CallToolRequest) -> CallToolResult {
    match parse_args(&req.arguments) {
        Ok((a, b)) => {
            let result = a * b;
            CallToolResult {
                content: vec![ContentBlock::Text(TextContent {
                    text: TextData::Text(result.to_string()),
                    options: None,
                })],
                is_error: None,
                meta: None,
                structured_content: None,
            }
        }
        Err(msg) => CallToolResult {
            content: vec![ContentBlock::Text(TextContent {
                text: TextData::Text(msg),
                options: None,
            })],
            is_error: Some(true),
            meta: None,
            structured_content: None,
        },
    }
}

fn execute_divide(req: &CallToolRequest) -> CallToolResult {
    match parse_args(&req.arguments) {
        Ok((a, b)) => {
            if b == 0.0 {
                return CallToolResult {
                    content: vec![ContentBlock::Text(TextContent {
                        text: TextData::Text("Division by zero".to_string()),
                        options: None,
                    })],
                    is_error: Some(true),
                    meta: None,
                    structured_content: None,
                };
            }

            let result = a / b;
            CallToolResult {
                content: vec![ContentBlock::Text(TextContent {
                    text: TextData::Text(result.to_string()),
                    options: None,
                })],
                is_error: None,
                meta: None,
                structured_content: None,
            }
        }
        Err(msg) => CallToolResult {
            content: vec![ContentBlock::Text(TextContent {
                text: TextData::Text(msg),
                options: None,
            })],
            is_error: Some(true),
            meta: None,
            structured_content: None,
        },
    }
}

// Helper functions

fn parse_args(arguments: &Option<String>) -> Result<(f64, f64), String> {
    let args_str = arguments.as_ref().ok_or_else(|| "Missing arguments".to_string())?;

    let json: serde_json::Value = serde_json::from_str(args_str)
        .map_err(|e| format!("Invalid JSON arguments: {}", e))?;

    let a = json.get("a")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "Missing or invalid parameter 'a'".to_string())?;

    let b = json.get("b")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "Missing or invalid parameter 'b'".to_string())?;

    Ok((a, b))
}

bindings::export!(Calculator with_types_in bindings);
