# Rust Weather Example

A Rust MCP handler that demonstrates:
- Echo tool for basic testing
- Weather tool with async HTTP requests using spin-sdk
- Works with both Spin and wasmtime runtimes
- Clean project structure with NO WIT files needed

## Quick Start

```bash
# Build and compose the component
make compose

# Run with Spin
spin up

# OR run with wasmtime
wasmtime serve -S cli -S http composed.wasm
```

## Testing the Tools

Test the echo tool:
```bash
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "echo",
      "arguments": {"message": "Hello!"}
    }
  }'
```

Test the weather tool:
```bash
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "weather",
      "arguments": {"location": "San Francisco"}
    }
  }'
```

## How It Works

1. **Handler**: The Rust handler implements MCP tools using the `wasmcp` SDK
2. **Gateway**: The pre-built gateway component (`wasmcp-spin.wasm`) handles HTTP and runtime integration
3. **Composition**: `wac plug` combines the handler and gateway into a single component (`composed.wasm`)
4. **Runtime**: The composed component runs on any WASI-compliant runtime (Spin, wasmtime, etc.)

The workflow is completely automated - no manual intervention needed between template and running server!

## Clean Project Structure

This example uses `wasmcp@0.2.7` which uses a proc macro to embed all WIT definitions. You don't need any WIT files in your project - everything is handled by the macro!

## Implementing Your Own Tools

### Sync Tools

For simple synchronous tools, implement the `ToolHandler` trait:

```rust
struct MyTool;

impl wasmcp::ToolHandler for MyTool {
    const NAME: &'static str = "my_tool";
    const DESCRIPTION: &'static str = "Description of my tool";
    
    fn input_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "param": { "type": "string" }
            },
            "required": ["param"]
        })
    }
    
    fn execute(args: serde_json::Value) -> Result<String, String> {
        // Your tool logic here
        Ok("Result".to_string())
    }
}
```

### Async Tools

For tools that need to make HTTP requests or other async operations, implement `AsyncToolHandler`:

```rust
struct MyAsyncTool;

impl wasmcp::AsyncToolHandler for MyAsyncTool {
    const NAME: &'static str = "my_async_tool";
    const DESCRIPTION: &'static str = "An async tool";
    
    fn input_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string" }
            },
            "required": ["url"]
        })
    }
    
    async fn execute_async(args: serde_json::Value) -> Result<String, String> {
        // Use spin_sdk for HTTP requests
        use spin_sdk::http::{Request, send};
        
        let url = args["url"].as_str().ok_or("Missing url")?;
        let request = Request::get(url);
        let response = send(request).await
            .map_err(|e| format!("Request failed: {:?}", e))?;
        
        Ok(String::from_utf8_lossy(response.body()).to_string())
    }
}
```

### Register Your Tools

Add your tools to the `mcp_handler` macro:

```rust
#[wasmcp::mcp_handler(
    tools(MyTool, MyAsyncTool, EchoTool, WeatherTool),
)]
mod handler {}
```

That's it! No WIT files, no boilerplate, just like spin-sdk!