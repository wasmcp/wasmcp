# weather-rs

An MCP server written in Rust

## Quick Start

```bash
make setup  # Install dependencies and verify environment
make build  # Build the MCP server (no auth)
make serve  # Run the server (default: wasmtime on port 8080)
```

With OAuth 2.0 authentication:
```bash
make build-auth  # Build with OAuth/JWT authorization
make serve-auth  # Run with auth (configure JWT env vars)
```

Test the server:
```bash
make test-all  # Run all tests
```

## Architecture

This MCP server runs as a WebAssembly component, combining:
- **Provider**: Your Rust implementation of MCP tools (this code)
- **Transport**: Pre-built HTTP server component from the registry
- **Authorization** (optional): OAuth 2.0/JWT validation component

The composition happens at build time, producing either:
- `mcp-http-server.wasm` - Basic server without authentication
- `mcp-http-auth-server.wasm` - Server with OAuth 2.0 authorization

## Development

### Prerequisites

- **Rust 1.89+** - Required for async traits and 2024 edition
- **cargo-component** - Compiles Rust to Wasm components
- **wasm-tools** - Component model toolchain
- **Spin SDK** - Provides async runtime and HTTP client

Quick setup:
```bash
make setup  # Checks and installs all dependencies
```

### Project Structure

```
├── src/
│   ├── lib.rs       # Tool implementations
│   └── helpers.rs   # MCP SDK-like async traits
├── wit/             # WebAssembly Interface Types
├── Cargo.toml       # Rust dependencies
└── Makefile         # Build automation
```

### Build Pipeline

The build process has two stages:

```bash
cargo component build    # Compile Rust to Wasm component
make build              # Compose with transport
```

Or simply: `make build` (runs all steps)

### Adding New Tools

Define your arguments as a struct and implement the `Tool` trait:

```rust
use helpers::{Tool, ToolResult, McpError, IntoToolResult, parse_args};
use serde::Deserialize;
use serde_json::json;

// Define typed arguments
#[derive(Deserialize)]
struct MyToolArgs {
    param: String,
    count: Option<u32>,  // Optional fields supported
}

struct MyTool;

impl Tool for MyTool {
    const NAME: &'static str = "my_tool";
    const DESCRIPTION: &'static str = "Tool description";
    
    fn input_schema() -> String {
        json!({
            "type": "object",
            "properties": {
                "param": {"type": "string", "description": "Parameter"},
                "count": {"type": "integer", "description": "Optional count"}
            },
            "required": ["param"]
        }).to_string()
    }
    
    async fn execute(args: Option<String>) -> Result<ToolResult, McpError> {
        // Type-safe argument parsing
        let args: MyToolArgs = parse_args(&args)?;
        
        // Tool implementation with direct field access
        let result = format!("Processing {} (count: {:?})", args.param, args.count);
        Ok(result.into_result())
    }
}
```

Then register it with the `register_tools!` macro:

```rust
register_tools!(EchoTool, WeatherTool, MyTool);
```

The approach provides:
- **Type safety**: Arguments are validated at deserialization
- **Native async**: Using Rust 1.75+ async traits
- **Clean API**: Direct field access on typed structs
- **Error handling**: Automatic validation and error messages

## Concurrency in Rust/Wasm

This template uses the Spin SDK, which provides an excellent async runtime implementation for Wasm components. The SDK bridges Rust's async/await to WASI's polling mechanism:

```rust
use spin_sdk::http::{Request, send};

// Concurrent HTTP requests
let futures = cities.iter().map(|city| {
    Box::pin(get_weather_for_city(city.clone()))
});

let results = futures::future::join_all(futures).await;
```

How it works:
1. **Spin SDK runtime**: Provides the async executor that maps to WASI polling
2. **`spin_sdk::http::send`**: Async HTTP client using WASI outbound-http
3. **No Send requirements**: Wasm is single-threaded, avoiding Rust's Send complications
4. **True concurrency**: Multiple HTTP requests run in parallel via WASI host polling

The template uses native `async fn` in traits (stable since Rust 1.75) for clean, idiomatic async APIs.

## Testing

The Makefile includes comprehensive test targets:

```bash
make test-all        # Run all tests
make test-echo       # Test echo tool
make test-weather    # Test weather tool  
make test-multi      # Test concurrent weather fetching
```

Tests use `curl` to send JSON-RPC requests to the running server. Example:

```bash
# Manual test
curl -X POST http://localhost:8080/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello"}},"id":1}'
```

## Code Quality

This template enforces strict code quality standards:

### Linting & Formatting

```bash
cargo fmt              # Format code with rustfmt
cargo clippy          # Run clippy with pedantic lints
cargo doc             # Generate documentation
```

The project uses:
- **Rust 2024 edition** for latest language features
- **Clippy pedantic mode** for comprehensive linting
- **rustfmt** with strict formatting rules
- **Documentation enforcement** for public APIs

### Cargo.toml Configuration

- `missing_docs = "warn"` - Documentation encouraged
- Clippy pedantic lints enabled with sensible overrides
- Rust 2024 edition with modern async trait support

## Debugging

### Common Issues

**Send trait errors**
- Wasm is single-threaded, no Send requirements
- Use `Box::pin()` for futures without Send bounds

**Linting errors**
- Run `cargo fmt` to fix formatting
- Address clippy warnings with `cargo clippy --fix`

**Server doesn't start**
- Verify port 8080 is available: `lsof -i :8080`
- Check wasmtime is installed: `which wasmtime`

### Inspecting the Component

```bash
make inspect  # Show component structure and exports
```

## Runtime Options

The server can run on component model runtimes:

```bash
# Wasmtime (default, no auth)
wasmtime serve -Scli ./mcp-http-server.wasm

# Wasmtime with OAuth authentication
export JWT_ISSUER="https://your-domain.authkit.app"
export JWT_AUDIENCE="client_YOUR_CLIENT_ID"
export JWT_JWKS_URI="https://your-domain.authkit.app/oauth2/jwks"
make serve-auth

# Spin (no auth only)
spin up
```

## OAuth 2.0 Authentication

The template includes optional OAuth 2.0/JWT authentication support:

```bash
# Build with auth
make build-auth

# Configure JWT validation (example with AuthKit)
export JWT_ISSUER="https://your-domain.authkit.app"
export JWT_AUDIENCE="client_YOUR_CLIENT_ID"
export JWT_JWKS_URI="https://your-domain.authkit.app/oauth2/jwks"

# Run with auth
make serve-auth
```

The auth-enabled server provides:
- JWT token validation with JWKS support
- OAuth 2.0 discovery endpoints (`/.well-known/oauth-*`)
- Configurable OPA/Rego policies for fine-grained access control
- Integration with enterprise auth providers (AuthKit, Auth0, etc.)

Test auth enforcement:
```bash
# Without token (returns 401)
make test-auth-no-token

# Check OAuth discovery
make test-auth-discovery
```