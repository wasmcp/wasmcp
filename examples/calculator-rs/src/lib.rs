//! Calculator Tools Capability Provider
//!
//! A tools capability that provides basic calculator operations with notification support.

mod bindings {
    wit_bindgen::generate!({
        world: "calculator",
        generate_all,
    });
}

use bindings::exports::wasmcp::protocol::tools::Guest;
use bindings::wasmcp::protocol::mcp::*;
use bindings::wasi::io::streams::OutputStream;

struct Calculator;

impl Guest for Calculator {
    fn list_tools(
        _ctx: bindings::wasmcp::protocol::server_messages::Context,
        _request: ListToolsRequest,
        _client_stream: Option<&OutputStream>,
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
        ctx: bindings::wasmcp::protocol::server_messages::Context,
        request: CallToolRequest,
        client_stream: Option<&OutputStream>,
    ) -> Option<CallToolResult> {
        match request.name.as_str() {
            "add" => Some(execute_operation(&request.arguments, |a, b| a + b)),
            "subtract" => Some(execute_operation(&request.arguments, |a, b| a - b)),
            "factorial" => Some(execute_factorial(&ctx, &request, client_stream)),
            _ => None, // We don't handle this tool
        }
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

// Calculates factorial with SSE notifications demonstrating progress updates.
// Note: When running with wasmtime serve, use `-S http-outgoing-body-buffer-chunks=10`
// to ensure sufficient buffer for streaming multiple notifications plus the final response.
fn execute_factorial(
    _ctx: &bindings::wasmcp::protocol::server_messages::Context,
    request: &CallToolRequest,
    client_stream: Option<&OutputStream>,
) -> CallToolResult {
    // Parse the argument to get n
    let n = match parse_factorial_arg(&request.arguments) {
        Ok(n) => n,
        Err(msg) => {
            return error_result(msg);
        }
    };

    // Send initial progress notification if stream is available
    if let Some(stream) = client_stream {
        send_log_notification(stream, &format!("Starting factorial calculation for {}!", n), "info", Some("factorial"));
    }

    // Calculate factorial with progress updates
    let mut result: u64 = 1;
    for i in 1..=n {
        match result.checked_mul(i) {
            Some(val) => result = val,
            None => {
                return error_result(format!("Integer overflow: {}! is too large", n));
            }
        }

        // Send progress notification every few steps (to avoid overwhelming)
        if let Some(stream) = client_stream {
            if i % 3 == 0 || i == n {
                send_log_notification(
                    stream,
                    &format!("Computing: {} * {} = {}", i, result / i, result),
                    "debug",
                    Some("factorial"),
                );
            }
        }
    }

    // Send completion notification
    if let Some(stream) = client_stream {
        send_log_notification(stream, &format!("Factorial calculation complete: {}! = {}", n, result), "info", Some("factorial"));
    }

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

// Helper functions for sending notifications via SSE

fn send_log_notification(
    stream: &OutputStream,
    message: &str,
    level: &str,
    logger: Option<&str>,
) {
    let notification = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/message",
        "params": {
            "level": level,
            "logger": logger,
            "data": message,
        }
    });

    write_sse_event(stream, &notification);
}

fn send_progress_notification(
    stream: &OutputStream,
    token: &str,
    progress: f64,
    total: Option<f64>,
    message: Option<&str>,
) {
    let mut params = serde_json::json!({
        "progressToken": token,
        "progress": progress,
    });

    if let Some(t) = total {
        params["total"] = serde_json::json!(t);
    }

    if let Some(msg) = message {
        params["message"] = serde_json::json!(msg);
    }

    let notification = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/progress",
        "params": params,
    });

    write_sse_event(stream, &notification);
}

fn write_sse_event(stream: &OutputStream, data: &serde_json::Value) {
    // Format as SSE exactly like http-transport does
    if let Ok(json_str) = serde_json::to_string(data) {
        let event_data = format!("data: {}\n\n", json_str);
        let bytes = event_data.as_bytes();

        // Write using check_write() to respect budget (like http-transport)
        let mut offset = 0;
        while offset < bytes.len() {
            match stream.check_write() {
                Ok(0) => break, // No budget available
                Ok(budget) => {
                    let chunk_size = (bytes.len() - offset).min(budget as usize);
                    let chunk = &bytes[offset..offset + chunk_size];
                    if stream.write(chunk).is_err() {
                        break;
                    }
                    offset += chunk_size;
                }
                Err(_) => break,
            }
        }
    }
}

bindings::export!(Calculator with_types_in bindings);
