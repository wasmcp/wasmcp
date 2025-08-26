# wasmcp

Rust SDK for MCP WebAssembly components. Zero WIT files, just a proc macro.

[![Crates.io](https://img.shields.io/crates/v/wasmcp.svg)](https://crates.io/crates/wasmcp)
[![docs.rs](https://docs.rs/wasmcp/badge.svg)](https://docs.rs/wasmcp)

## Installation

```toml
[dependencies]
wasmcp = "0.2"
```

## Usage

```rust
use wasmcp::{mcp_handler, ToolHandler, AsyncToolHandler};

// Sync tool
struct EchoTool;

impl ToolHandler for EchoTool {
    const NAME: &'static str = "echo";
    const DESCRIPTION: &'static str = "Echo a message";
    
    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" }
            },
            "required": ["message"]
        })
    }
    
    fn execute(args: serde_json::Value) -> Result<String, String> {
        Ok(format!("Echo: {}", args["message"]))
    }
}

// Async tool
struct WeatherTool;

impl AsyncToolHandler for WeatherTool {
    const NAME: &'static str = "weather";
    const DESCRIPTION: &'static str = "Get weather for a location";
    
    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "location": { "type": "string" }
            },
            "required": ["location"]
        })
    }
    
    async fn execute_async(args: serde_json::Value) -> Result<String, String> {
        use spin_sdk::http::{Request, send};
        
        let location = args["location"].as_str().ok_or("Missing location")?;
        let request = Request::get(&format!("https://api.weather.com?q={}", location));
        let response = send(request).await
            .map_err(|e| format!("Request failed: {:?}", e))?;
        
        Ok(format!("Weather data for {}", location))
    }
}

// Register your tools - no WIT files needed!
#[mcp_handler(
    tools(EchoTool, WeatherTool),
)]
mod handler {}
```

## Features

- **No WIT files**: Proc macro embeds all WebAssembly interface definitions
- **Full async support**: Both sync and async tools work seamlessly
- **Zero overhead**: Compile-time dispatch, no vtables
- **Type safe**: Leverage Rust's type system
- **Spin SDK integration**: Use `spin_sdk` for HTTP, KV, and more

## Resources

```rust
struct ConfigResource;

impl ResourceHandler for ConfigResource {
    const URI: &'static str = "config://app";
    const NAME: &'static str = "Application Config";
    const MIME_TYPE: Option<&'static str> = Some("application/json");
    
    fn read() -> Result<String, String> {
        Ok(r#"{"version": "1.0.0"}"#.to_string())
    }
}

// Add to handler
#[mcp_handler(
    tools(EchoTool),
    resources(ConfigResource),
)]
mod handler {}
```

## Prompts

```rust
struct GreetingPrompt;

impl PromptHandler for GreetingPrompt {
    const NAME: &'static str = "greeting";
    const DESCRIPTION: Option<&'static str> = Some("Generate greeting");
    
    type Arguments = GreetingArgs;
    
    fn resolve(args: serde_json::Value) -> Result<Vec<PromptMessage>, String> {
        Ok(vec![
            PromptMessage {
                role: PromptRole::Assistant,
                content: format!("Hello, {}!", args["name"]),
            }
        ])
    }
}

// Add to handler
#[mcp_handler(
    tools(EchoTool),
    prompts(GreetingPrompt),
)]
mod handler {}
```

## Building

```bash
cargo component build --release --target wasm32-wasip2
```

The resulting WASM component works with any MCP gateway (wasmcp-spin, custom, etc).

## Performance

- **Zero-sized types**: Tools are just type markers
- **Const metadata**: All strings are `&'static str`
- **Compile-time dispatch**: No runtime overhead
- **Automatic async runtime**: `spin_executor` handles async/await

## License

Apache-2.0