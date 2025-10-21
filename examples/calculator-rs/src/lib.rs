//! Calculator Tools Capability Provider
//!
//! A tools capability that provides basic calculator operations.
//! This example demonstrates how to use MCP notifications to send
//! logs and progress updates back to the client.

mod bindings {
    wit_bindgen::generate!({
        world: "calculator",
        generate_all,
    });
}

use bindings::exports::wasmcp::server::tools::Guest;
use bindings::wasmcp::protocol::mcp::*;
use bindings::wasmcp::server::notifications::{NotificationChannel, LogLevel};

struct Calculator;

impl Guest for Calculator {
    fn list_tools(
        _ctx: bindings::wasmcp::server::server_messages::Context,
        _request: ListToolsRequest,
        channel: Option<&NotificationChannel>,
    ) -> Result<ListToolsResult, ErrorCode> {
        // Send a debug log when tools are being listed
        if let Some(ch) = channel {
            let _ = ch.log(
                "Listing available calculator tools",
                LogLevel::Debug,
                Some("calculator"),
            );
        }

        Ok(ListToolsResult {
            tools: vec![
                Tool {
                    name: "add".to_string(),
                    input_schema: r#"{
                        "type": "object",
                        "properties": {
                            "a": {"type": "number", "description": "First number"},
                            "b": {"type": "number", "description": "Second number"}
                        },
                        "required": ["a", "b"]
                    }"#
                    .to_string(),
                    options: Some(ToolOptions {
                        meta: None,
                        annotations: None,
                        description: Some("Add two numbers together".to_string()),
                        output_schema: None,
                        title: Some("Add".to_string()),
                    }),
                },
                Tool {
                    name: "subtract".to_string(),
                    input_schema: r#"{
                        "type": "object",
                        "properties": {
                            "a": {"type": "number", "description": "Number to subtract from"},
                            "b": {"type": "number", "description": "Number to subtract"}
                        },
                        "required": ["a", "b"]
                    }"#
                    .to_string(),
                    options: None,
                },
            ],
            next_cursor: None,
            meta: None,
        })
    }

    fn call_tool(
        _ctx: bindings::wasmcp::server::server_messages::Context,
        request: CallToolRequest,
        channel: Option<&NotificationChannel>,
    ) -> Option<CallToolResult> {
        // Send a log notification when starting a calculation
        if let Some(ch) = channel {
            let _ = ch.log(
                &format!("Starting {} calculation", request.name),
                LogLevel::Info,
                Some("calculator"),
            );
        }

        let result = match request.name.as_str() {
            "add" => Some(execute_operation(
                &request.arguments,
                |a, b| a + b,
                "add",
                channel,
            )),
            "subtract" => Some(execute_operation(
                &request.arguments,
                |a, b| a - b,
                "subtract",
                channel,
            )),
            _ => {
                // Log when we receive an unknown tool request
                if let Some(ch) = channel {
                    let _ = ch.log(
                        &format!("Unknown tool requested: {}", request.name),
                        LogLevel::Warning,
                        Some("calculator"),
                    );
                }
                None // We don't handle this tool
            }
        };

        // Send completion log if we handled the request
        if result.is_some() {
            if let Some(ch) = channel {
                let _ = ch.log(
                    &format!("Completed {} calculation", request.name),
                    LogLevel::Debug,
                    Some("calculator"),
                );
            }
        }

        result
    }
}

fn execute_operation<F>(
    arguments: &Option<String>,
    op: F,
    operation_name: &str,
    channel: Option<&NotificationChannel>,
) -> CallToolResult
where
    F: FnOnce(f64, f64) -> f64,
{
    match parse_args(arguments) {
        Ok((a, b)) => {
            // Send debug log with the actual calculation being performed
            if let Some(ch) = channel {
                let expression = match operation_name {
                    "add" => format!("{} + {}", a, b),
                    "subtract" => format!("{} - {}", a, b),
                    _ => format!("{}({}, {})", operation_name, a, b),
                };
                let _ = ch.log(
                    &format!("Calculating: {}", expression),
                    LogLevel::Debug,
                    Some("calculator"),
                );
            }

            let result = op(a, b);

            // Send info log with the result
            if let Some(ch) = channel {
                let expression = match operation_name {
                    "add" => format!("{} + {} = {}", a, b, result),
                    "subtract" => format!("{} - {} = {}", a, b, result),
                    _ => format!("{}({}, {}) = {}", operation_name, a, b, result),
                };
                let _ = ch.log(
                    &format!("Result: {}", expression),
                    LogLevel::Info,
                    Some("calculator"),
                );
            }

            success_result(result.to_string())
        }
        Err(msg) => {
            // Send error log when calculation fails
            if let Some(ch) = channel {
                let _ = ch.log(
                    &format!("Calculation error: {}", msg),
                    LogLevel::Error,
                    Some("calculator"),
                );
            }
            error_result(msg)
        }
    }
}

fn parse_args(arguments: &Option<String>) -> Result<(f64, f64), String> {
    let args_str = arguments
        .as_ref()
        .ok_or_else(|| "Missing arguments".to_string())?;

    let json: serde_json::Value =
        serde_json::from_str(args_str).map_err(|e| format!("Invalid JSON arguments: {}", e))?;

    let a = json
        .get("a")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "Missing or invalid parameter 'a'".to_string())?;

    let b = json
        .get("b")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "Missing or invalid parameter 'b'".to_string())?;

    Ok((a, b))
}

fn success_result(result: String) -> CallToolResult {
    CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: TextData::Text(result),
            options: None,
        })],
        is_error: None,
        meta: None,
        structured_content: None,
    }
}

fn error_result(message: String) -> CallToolResult {
    CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: TextData::Text(message),
            options: None,
        })],
        is_error: Some(true),
        meta: None,
        structured_content: None,
    }
}

bindings::export!(Calculator with_types_in bindings);
