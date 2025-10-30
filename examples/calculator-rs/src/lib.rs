//! Calculator Tools Capability Provider
//!
//! A tools capability that provides basic calculator operations with notification support.

mod bindings {
    wit_bindgen::generate!({
        world: "calculator",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp_v20250618::tools::Guest;
use bindings::wasmcp::mcp_v20250618::mcp::*;
use bindings::wasmcp::mcp_v20250618::server_messages;
use bindings::wasmcp::mcp_v20250618::server_handler::RequestCtx;

struct Calculator;

impl Guest for Calculator {
    fn list_tools(
        _ctx: RequestCtx,
        _request: ListToolsRequest,
    ) -> Result<ListToolsResult, ErrorCode> {

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
                Tool {
                    name: "factorial".to_string(),
                    input_schema: r#"{
                        "type": "object",
                        "properties": {
                            "n": {
                                "type": "integer",
                                "description": "Calculate factorial of this number",
                                "minimum": 0,
                                "maximum": 20
                            }
                        },
                        "required": ["n"]
                    }"#
                    .to_string(),
                    options: Some(ToolOptions {
                        meta: None,
                        annotations: None,
                        description: Some("Calculate factorial with progress updates".to_string()),
                        output_schema: None,
                        title: Some("Factorial".to_string()),
                    }),
                },
            ],
            next_cursor: None,
            meta: None,
        })
    }

    fn call_tool(
        ctx: RequestCtx,
        request: CallToolRequest,
    ) -> Result<Option<CallToolResult>, ErrorCode> {
        let result = match request.name.as_str() {
            "add" => Some(execute_operation(&request.arguments, |a, b| a + b)),
            "subtract" => Some(execute_operation(&request.arguments, |a, b| a - b)),
            "factorial" => Some(execute_factorial(&ctx, &request)),
            _ => None, // We don't handle this tool
        };
        Ok(result)
    }
}

fn execute_operation<F>(arguments: &Option<String>, op: F) -> CallToolResult
where
    F: FnOnce(f64, f64) -> f64,
{
    match parse_args(arguments) {
        Ok((a, b)) => {
            let result = op(a, b);
            success_result(result.to_string())
        }
        Err(msg) => error_result(msg),
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

// Calculates factorial with SSE server_messages demonstrating progress updates.
// Note: When running with wasmtime serve, use `-S http-outgoing-body-buffer-chunks=10`
// to ensure sufficient buffer for streaming multiple server_messages plus the final response.
fn execute_factorial(
    ctx: &RequestCtx,
    request: &CallToolRequest,
) -> CallToolResult {
    // Parse the argument to get n
    let n = match parse_factorial_arg(&request.arguments) {
        Ok(n) => n,
        Err(msg) => {
            return error_result(msg);
        }
    };

    let log = |msg| if let Some(stream) = ctx.messages {
        let _ = server_messages::notify(
            stream,
            &ServerNotification::Log(LoggingMessageNotification {
                data: msg,
                level: LogLevel::Info,
                logger: Some("factorial".to_string())
            })
        );
    };

    // Send initial progress notification if stream is available
    log(format!("Starting factorial calculation for {n}!"));

    // Calculate factorial with progress updates
    let mut result: u64 = 1;
    for i in 1..=n {
        match result.checked_mul(i) {
            Some(val) => result = val,
            None => {
                return error_result(format!("Integer overflow: {n}! is too large"));
            }
        }

        // Send progress notification every few steps (to avoid overwhelming)
        if i % 3 == 0 || i == n {
            log(format!("Computing: {i} * {} = {result}", result / i));
        }
    }

    // Send completion notification
    log(format!("Factorial calculation complete: {n}! = {result}"));

    success_result(result.to_string())
}

fn parse_factorial_arg(arguments: &Option<String>) -> Result<u64, String> {
    let args_str = arguments
        .as_ref()
        .ok_or_else(|| "Missing arguments".to_string())?;

    let json: serde_json::Value =
        serde_json::from_str(args_str).map_err(|e| format!("Invalid JSON arguments: {}", e))?;

    let n = json
        .get("n")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "Missing or invalid parameter 'n'".to_string())?;

    if n > 20 {
        return Err(format!("Input too large: {} (maximum is 20)", n));
    }

    Ok(n)
}

bindings::export!(Calculator with_types_in bindings);
