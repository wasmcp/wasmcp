# {{project-name | kebab_case}}

{{project-description}}

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
- HTTP transport v0.4.1 (from registry) - handles JSON-RPC over HTTP
- Optional OAuth 2.0 authorization

## Example Tools

This server implements three demonstration tools:

- **`echo`** - Simple message echo for testing
- **`get_weather`** - Fetch weather for a single location
- **`multi_weather`** - Concurrent weather fetching for multiple cities (demonstrates async with futures)

## Development

### Prerequisites

- Rust 1.89+
- cargo-component
- wac
- wkg

### Project Structure

```
src/lib.rs       # MCP capabilities implementation
wit/             # WIT interface definitions (wasmcp:mcp@0.1.0)
Cargo.toml       # Dependencies and component metadata
Makefile         # Build automation
```

### Implementing Tools

Tools are handled directly in the `handle_call_tool` function:

```rust
fn handle_call_tool(request: CallToolRequest) -> Result<ToolResult, McpError> {
    match request.name.as_str() {
        "echo" => spin_sdk::http::run(async move { 
            handle_echo(request.arguments).await 
        }),
        "get_weather" => spin_sdk::http::run(async move { 
            handle_get_weather(request.arguments).await 
        }),
        _ => Err(McpError::ToolNotFound),
    }
}

async fn handle_get_weather(args: Option<String>) -> Result<ToolResult, McpError> {
    let params: WeatherParams = serde_json::from_str(&args.unwrap_or_default())?;
    
    // Fetch weather data
    let weather = fetch_weather(&params.location).await?;
    Ok(text_result(&weather))
}
```

## Testing

```bash
make test-all        # Run all tests
make test-weather    # Test weather tool
make test-multi      # Test concurrent fetching
```

## Concurrency

Rust's Wasm environment uses `spin_sdk::http::run()` for async operations. Example from the multi-weather implementation:

```rust
async fn handle_multi_weather(args: Option<String>) -> Result<ToolResult, McpError> {
    let params: MultiWeatherParams = serde_json::from_str(&args.unwrap_or_default())?;
    
    // Concurrent HTTP requests using futures
    let futures = params.cities.iter().map(|city| fetch_weather(city));
    let results = futures::future::join_all(futures).await;
    
    // Format results
    let output = format_weather_results(results);
    Ok(text_result(&output))
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