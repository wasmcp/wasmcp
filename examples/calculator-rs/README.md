# Calculator Tools (Rust)

A foundational example demonstrating the tools capability pattern in wasmcp. Provides three basic math operations with progress notifications and tool metadata.

## Overview

The calculator-rs component shows how to:

- **Implement tools capability**: Export `wasmcp:mcp-v20250618/tools` interface
- **Define tool schemas**: JSON Schema for input validation
- **Send progress notifications**: Server-sent events (SSE) during tool execution
- **Add tool metadata**: Component ID and tags for filtering

This is the simplest working example of an MCP tools provider and serves as a template for creating your own tools.

## Tools Provided

| Tool | Description | Arguments |
|------|-------------|-----------|
| `add` | Add two numbers | `a: number, b: number` |
| `subtract` | Subtract two numbers | `a: number, b: number` |
| `factorial` | Calculate factorial with progress | `n: integer (0-20)` |

All tools include metadata tags:
```json
{
  "component_id": "calculator-rs",
  "tags": {
    "category": "math",
    "tool-level": "foundational"
  }
}
```

## Quick Start

```bash
# Build the component
make build

# Compose into MCP server
make compose

# Run with Spin
spin up

# In another terminal, test the tools
wasmcp mcp call-tool add '{"a": 5, "b": 3}'
# Result: 8

wasmcp mcp call-tool factorial '{"n": 5}'
# Result: 120 (with progress notifications)
```

## Building

```bash
# Install dependencies
make setup

# Update WIT dependencies
make wit

# Build the component
make build
# Creates: target/wasm32-wasip2/release/calculator.wasm

# Compose into complete MCP server
make compose
# Creates: mcp-server.wasm
```

## Implementation Guide

### 1. Define Your Component World

```wit
// wit/world.wit
package wasmcp:calculator@0.1.0;

world calculator {
    // Import server-io for sending notifications
    import wasmcp:mcp-v20250618/server-io@0.1.7;

    // Export tools capability
    export wasmcp:mcp-v20250618/tools@0.1.7;
}
```

**Note**: MessageContext is automatically provided to your tool functions through the tools interface - no explicit import needed.

### 2. Implement the Tools Interface

The tools capability has two required methods:

#### `list_tools` - Advertise Available Tools

```rust
use bindings::exports::wasmcp::mcp_v20250618::tools::Guest;
use bindings::wasmcp::mcp_v20250618::mcp::*;
use bindings::wasmcp::mcp_v20250618::server_handler::MessageContext;

struct Calculator;

impl Guest for Calculator {
    fn list_tools(
        _ctx: MessageContext,
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
                    }"#.to_string(),
                    options: Some(ToolOptions {
                        description: Some("Add two numbers together".to_string()),
                        title: Some("Add".to_string()),
                        meta: Some(r#"{
                            "component_id": "my-component",
                            "tags": {
                                "category": "math",
                                "tool-level": "foundational"
                            }
                        }"#.to_string()),
                        // ... other options
                    }),
                },
                // More tools...
            ],
            next_cursor: None,
            meta: None,
        })
    }

    // Implement call_tool next...
}
```

**Key points**:
- **input_schema**: JSON Schema string defining tool arguments
- **description**: Human-readable description of what the tool does
- **meta**: JSON object with component_id and tags (used by filter-middleware)

#### `call_tool` - Execute Tool Logic

```rust
fn call_tool(
    ctx: MessageContext,
    request: CallToolRequest,
) -> Result<Option<CallToolResult>, ErrorCode> {
    let result = match request.name.as_str() {
        "add" => {
            // Parse arguments
            let (a, b) = parse_args(&request.arguments)?;

            // Execute operation
            let result = a + b;

            // Return success result
            Some(success_result(result.to_string()))
        }
        "subtract" => {
            let (a, b) = parse_args(&request.arguments)?;
            Some(success_result((a - b).to_string()))
        }
        _ => None, // Tool not handled by this component
    };

    Ok(result)
}
```

**Key points**:
- Return `Ok(Some(...))` if you handle the tool
- Return `Ok(None)` if tool name doesn't match (allows composition)
- Parse `request.arguments` as JSON string
- Validate inputs and return errors via `is_error: Some(true)`

### 3. Argument Parsing

```rust
fn parse_args(arguments: &Option<String>) -> Result<(f64, f64), String> {
    let args_str = arguments
        .as_ref()
        .ok_or_else(|| "Missing arguments".to_string())?;

    let json: serde_json::Value = serde_json::from_str(args_str)
        .map_err(|e| format!("Invalid JSON arguments: {}", e))?;

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
```

### 4. Result Construction

```rust
fn success_result(result: String) -> CallToolResult {
    CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: TextData::Text(result),
            options: None,
        })],
        is_error: None,  // or Some(false)
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
        is_error: Some(true),  // Mark as error
        meta: None,
        structured_content: None,
    }
}
```

### 5. Progress Notifications (Optional)

Send server notifications during long-running operations:

```rust
use bindings::wasmcp::mcp_v20250618::server_io;

fn execute_factorial(
    ctx: &MessageContext,
    request: &CallToolRequest,
) -> CallToolResult {
    let n = parse_factorial_arg(&request.arguments)?;

    // Helper to send log notifications
    let log = |msg: String| {
        if let Some(stream) = ctx.client_stream {
            let _ = server_io::send_message(
                stream,
                ServerMessage::Notification(
                    ServerNotification::Log(LoggingMessageNotification {
                        data: msg,
                        level: LogLevel::Info,
                        logger: Some("factorial".to_string()),
                    })
                ),
                &ctx.frame
            );
        }
    };

    log(format!("Starting factorial calculation for {n}!"));

    let mut result: u64 = 1;
    for i in 1..=n {
        result *= i;

        // Progress update every few steps
        if i % 3 == 0 || i == n {
            log(format!("Computing step {i}: result = {result}"));
        }
    }

    log(format!("Calculation complete: {n}! = {result}"));

    success_result(result.to_string())
}
```

**Notes**:
- `ctx.client_stream` is only available in HTTP transport (not stdio)
- Notifications are best-effort (failures are silently ignored)
- Use sparingly to avoid overwhelming clients

## Composing with Other Components

Calculator can be composed with middleware and other tools:

### With Filter Middleware

```bash
wasmcp compose server \
  filter_middleware.wasm \
  routing_config.wasm \
  calculator.wasm \
  -o mcp-server.wasm
```

Filter-middleware will use the `component_id` and `tags` metadata to apply routing rules.

### With Counter Middleware

```bash
wasmcp compose server \
  counter_middleware.wasm \
  calculator.wasm \
  -o mcp-server.wasm
```

Counter-middleware will track how many times each calculator tool is called.

### With Other Tools

```bash
wasmcp compose server \
  calculator.wasm \
  weather.wasm \
  todo_list.wasm \
  -o mcp-server.wasm
```

All tools from all components are available at the same MCP endpoint.

## Testing

### With wasmcp CLI

```bash
# Start server
spin up

# Initialize session
wasmcp mcp initialize

# List available tools
wasmcp mcp list-tools

# Call tools
wasmcp mcp call-tool add '{"a": 10, "b": 5}'
# Response: 15

wasmcp mcp call-tool subtract '{"a": 10, "b": 3}'
# Response: 7

wasmcp mcp call-tool factorial '{"n": 5}'
# Response: 120
# (with progress notifications if using HTTP transport)
```

### With curl

```bash
# Initialize session
RESPONSE=$(curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -D - \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}')

# Extract session ID
SESSION_ID=$(echo "$RESPONSE" | grep -i "mcp-session-id" | cut -d' ' -f2 | tr -d '\r')

# Call a tool
curl -X POST http://localhost:3000/mcp \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"add","arguments":"{\"a\":5,\"b\":3}"}}'
```

## Integration with Claude Code

Add to Claude Code MCP server configuration:

```json
{
  "mcpServers": {
    "calculator": {
      "command": "spin",
      "args": [
        "up",
        "--listen",
        "127.0.0.1:3000",
        "--from",
        "/path/to/calculator-rs"
      ]
    }
  }
}
```

Claude Code can then use the calculator tools:

```
User: What is 42 plus 17?
Claude: I'll use the add tool to calculate that.
[calls add tool with {"a": 42, "b": 17}]
Claude: 42 + 17 = 59
```

## Advanced Patterns

### Structured Content

Return structured data in addition to text:

```rust
fn success_with_structured(result: f64, operation: &str) -> CallToolResult {
    CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: TextData::Text(format!("Result: {}", result)),
            options: None,
        })],
        structured_content: Some(serde_json::json!({
            "result": result,
            "operation": operation,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }).to_string()),
        is_error: None,
        meta: None,
    }
}
```

### Tool Annotations

Add MCP annotations for tool behavior hints:

```rust
Tool {
    name: "add".to_string(),
    input_schema: /* ... */,
    options: Some(ToolOptions {
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(true),  // Tool doesn't modify state
            idempotent_hint: Some(true), // Same inputs → same output
            // ...
        }),
        // ... other options
    }),
}
```

### Custom Metadata

Add custom metadata for your application:

```rust
Tool {
    name: "add".to_string(),
    // ...
    options: Some(ToolOptions {
        meta: Some(serde_json::json!({
            "component_id": "calculator-rs",
            "tags": {
                "category": "math",
                "tool-level": "foundational",
                "cost-tier": "free",
                "rate-limit": "1000/hour"
            },
            "version": "1.0.0",
            "author": "example-team"
        }).to_string()),
        // ...
    }),
}
```

## Files

```
calculator-rs/
├── Cargo.toml           # Rust package configuration
├── Makefile             # Build targets
├── README.md            # This file
├── spin.toml            # Spin runtime configuration
├── wit/
│   ├── deps/            # WIT dependencies
│   ├── deps.lock
│   ├── deps.toml
│   └── world.wit       # Component world definition
└── src/
    └── lib.rs          # Tool implementation
```

## Related Examples

- **todo-list-auth** - Tools with authorization patterns
- **weather-ts** - Tools in TypeScript with HTTP requests
- **strings-py** - Tools in Python
- **counter-middleware** - Middleware pattern and session storage
- **routing-config** - Path-based tool filtering

## Related Documentation

- [MCP Tools Capability](https://spec.modelcontextprotocol.io/capabilities/tools/)
- [WebAssembly Component Model](https://component-model.bytecodealliance.org/)
- [JSON Schema](https://json-schema.org/)
