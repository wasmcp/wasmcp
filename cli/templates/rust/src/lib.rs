//! {{project_name}} - Tools Capability Provider
//!
//! A clean tools capability component that provides basic arithmetic operations.

#![allow(warnings)]

mod bindings {
    wit_bindgen::generate!({
        world: "{{project_name}}",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp::tools_capability::Guest;
use bindings::wasmcp::mcp::protocol::*;

struct Handler;

impl Guest for Handler {
    fn list_tools(
        request: ListToolsRequest,
        client: ClientContext,
    ) -> ListToolsResult {
        ListToolsResult {
            tools: vec![
                create_sum_tool(),
                create_sub_tool(),
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
            "sum" => Some(execute_sum(&request)),
            "sub" => Some(execute_sub(&request)),
            _ => None, // We don't handle this tool
        }
    }
}

// Tool Definitions
// ----------------

fn create_sum_tool() -> Tool {
    Tool {
        name: "sum".to_string(),
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
            title: Some("Sum".to_string()),
        }),
    }
}

fn create_sub_tool() -> Tool {
    Tool {
        name: "sub".to_string(),
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

// Tool Execution
// --------------

fn execute_sum(req: &CallToolRequest) -> CallToolResult {
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

fn execute_sub(req: &CallToolRequest) -> CallToolResult {
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

// Helper Functions
// ----------------

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

bindings::export!(Handler with_types_in bindings);
