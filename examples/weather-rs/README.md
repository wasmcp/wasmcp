# weather-rs

An MCP server written in Rust

## Quick Start

```bash
make setup  # Install dependencies and configure tools
make build  # Build and compose Wasm components
make serve  # Run server on port 8080
```

## Architecture

This implementation uses WIT bindings directly as the SDK, providing transparent access to the MCP protocol. The approach eliminates abstraction layers, making the protocol implementation explicit and debuggable.

Components composed at build time:
- Provider component (this code) - exports MCP capabilities
- HTTP transport v0.2.0 (from registry) - handles JSON-RPC over HTTP
- Optional OAuth 2.0 authorization

## Example Tools

This server implements three demonstration tools:

- **`echo`** - Simple message echo for testing
- **`get_weather`** - Fetch weather for a single location
- **`multi_weather`** - Concurrent weather fetching for multiple cities (demonstrates async with futures)

## Development

### Prerequisites

- Rust 1.89+
- cargo-component 0.18.0+
- wac (WebAssembly Composition)
- wkg (WebAssembly package manager)

### Project Structure

```
src/
├── lib.rs           # Component entry point and exports
├── authorization.rs # OAuth 2.0 configuration
├── lifecycle.rs     # Server initialization and lifecycle
├── tools.rs         # Tool implementations
└── bindings.rs      # Generated WIT bindings (auto-generated)
wit/                 # WIT interface definitions (wasmcp:mcp@0.2.0)
Cargo.toml           # Dependencies and component metadata
Makefile             # Build automation
setup.sh             # Initial setup script
```

### Implementing Tools

Tools are implemented in `src/tools.rs` using the Guest trait:

```rust
impl ToolsGuest for Component {
    fn list_tools(_request: ListToolsRequest) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult {
            tools: vec![
                Tool {
                    name: "get_weather".to_string(),
                    title: Some("get_weather".to_string()),
                    description: Some("Get current weather for a location".to_string()),
                    input_schema: Some(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "location": {
                                "type": "string",
                                "description": "City name to get weather for"
                            }
                        },
                        "required": ["location"]
                    }).to_string()),
                    icons: None,
                },
            ],
            next_cursor: None,
        })
    }

    fn call_tool(request: CallToolRequest, _context: Option<AuthContext>) -> Result<CallToolResult, McpError> {
        match request.name.as_str() {
            "get_weather" => {
                spin_sdk::http::run(async move { handle_get_weather(request.arguments).await })
            }
            _ => Err(McpError {
                code: ErrorCode::MethodNotFound,
                message: format!("Unknown tool: {}", request.name),
                data: None,
            }),
        }
    }
}
```

## Testing

```bash
make test-all        # Run all tests
make test-init       # Test initialization
make test-tools      # Test tools/list
make test-echo       # Test echo tool
make test-weather    # Test get_weather tool
make test-multi      # Test multi_weather tool
```

## Concurrency

Rust's Wasm environment uses `spin_sdk::http::run()` for async operations. Example from the multi-weather implementation:

```rust
async fn handle_multi_weather(args: Option<String>) -> Result<CallToolResult, McpError> {
    let args: MultiWeatherArgs = parse_args(args.as_ref())?;
    
    // Concurrent HTTP requests using futures
    let futures = args.cities.iter().map(|city| {
        let city = city.clone();
        async move {
            match get_weather_for_city(&city).await {
                Ok(weather) => format!("{weather}\n"),
                Err(e) => format!("Error fetching weather for {city}: {e}\n"),
            }
        }
    });
    
    let results = futures::future::join_all(futures).await;
    let mut output = String::from("=== Weather Results ===\n\n");
    for result in results {
        output.push_str(&result);
    }
    
    Ok(text_result(output))
}
```

## Authorization

OAuth 2.0 authorization is optional and configured in the `get_auth_config` function:

```rust
fn get_auth_config() -> Option<ProviderAuthConfig> {
    // Return None to disable authorization
    None
    
    // Or enable OAuth 2.0 protection:
    // Some(ProviderAuthConfig {
    //     expected_issuer: "https://your-domain.authkit.app".to_string(),
    //     expected_audiences: vec!["client_id".to_string()],
    //     jwks_uri: "https://your-domain.authkit.app/oauth2/jwks".to_string(),
    //     policy: None,      // Optional Rego policy string
    //     policy_data: None, // Optional policy data JSON
    // })
}
```

The transport component handles:
- JWT validation
- JWKS fetching and caching
- OAuth discovery endpoints
- Rego policy evaluation (if configured)

## Deployment

```bash
# Local development with Wasmtime
wasmtime serve -Scli mcp-http-server.wasm

# Spin framework
spin up --from mcp-http-server.wasm

# Deploy to Fermyon Cloud
spin cloud deploy
```

## License

Apache-2.0