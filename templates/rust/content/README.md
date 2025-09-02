# {{project-name}}

{{project-description}}

## Quick Start

```bash
make setup  # Install dependencies
make build  # Build server
make serve  # Run on port 8080
```

With OAuth authentication:
```bash
make build-auth
make serve-auth
```

## Architecture

WebAssembly components composed at build time:
- Provider component (this code)
- HTTP transport (from registry)
- Authorization (optional)

## Development

### Prerequisites

- Rust 1.89+
- cargo-component
- wasm-tools
- wac
- wkg

### Project Structure

```
src/
├── lib.rs       # Tool implementations
└── helpers.rs   # MCP SDK traits
```

### Adding Tools

Implement the `Tool` trait:

```rust
struct MyTool;

impl Tool for MyTool {
    const NAME: &'static str = "my_tool";
    const DESCRIPTION: &'static str = "Description";
    
    fn input_schema() -> String {
        json!({
            "type": "object",
            "properties": {
                "param": {"type": "string"}
            },
            "required": ["param"]
        }).to_string()
    }
    
    async fn execute(args: Option<String>) -> Result<ToolResult, McpError> {
        let args: MyToolArgs = parse_args(&args)?;
        Ok(format!("Result: {}", args.param).into_result())
    }
}
```

Register with `register_tools!` macro.

## Concurrency

The Spin SDK provides async runtime for Wasm:

```rust
let futures = cities.iter().map(|city| {
    Box::pin(get_weather_for_city(city.clone()))
});
let results = futures::future::join_all(futures).await;
```

## Testing

```bash
make test-all    # Run all tests
make test-echo   # Test echo tool
```

## OAuth Authentication

Optional OAuth 2.0/JWT support:

```bash
export JWT_ISSUER="https://auth.example.com"
export JWT_AUDIENCE="client_123"
export JWT_JWKS_URI="https://auth.example.com/.well-known/jwks.json"
make serve-auth
```

Features:
- JWT validation with JWKS
- OAuth discovery endpoints
- OPA/Rego policies
- Works with AuthKit, Auth0, etc.

## Runtime Options

```bash
# Wasmtime
wasmtime serve -Scli mcp-http-server.wasm

# Spin (no auth only)
spin up
```

## License

Apache-2.0