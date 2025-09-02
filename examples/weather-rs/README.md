# weather-rs

MCP server example in Rust.

## Quick Start

```bash
make setup  # Install dependencies
make build  # Build server (no auth)
make serve  # Run on port 8080
```

With OAuth authentication:
```bash
make build-auth
make serve-auth  # Configure JWT env vars first
```

## Architecture

WebAssembly components composed at build time:
- Provider component (this code)
- HTTP transport component (from registry)
- Authorization component (optional)

## Tools

- `echo` - Echo a message
- `get_weather` - Get weather for a location
- `multi_weather` - Get weather for multiple cities concurrently

## Development

### Prerequisites

- Rust 1.89+ (for async traits)
- cargo-component
- wasm-tools
- wac (for composition)
- wkg (for registry packages)

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

Register with:
```rust
register_tools!(EchoTool, WeatherTool, MyTool);
```

## Testing

```bash
make test-all        # Run all tests
make test-weather    # Test weather tool
make test-multi      # Test concurrent fetching
```

## OAuth Support

Configure JWT validation:
```bash
export JWT_ISSUER="https://your-domain.authkit.app"
export JWT_AUDIENCE="client_YOUR_CLIENT_ID"
export JWT_JWKS_URI="https://your-domain.authkit.app/oauth2/jwks"
make serve-auth
```

Test auth:
```bash
make test-auth-no-token    # Should return 401
make test-auth-discovery   # OAuth discovery endpoints
```

## License

Apache-2.0