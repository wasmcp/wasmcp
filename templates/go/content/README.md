# {{project-name}}

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
- **`multi_weather`** - Concurrent weather fetching for multiple cities (demonstrates wasihttp.GetConcurrently)

## Development

### Prerequisites

- Go 1.23+
- TinyGo 0.34+
- wit-bindgen-go
- wac
- wkg

### Project Structure

```
main.go          # MCP capabilities implementation
wit/             # WIT interface definitions (fastertools:mcp@0.4.1)
internal/        # Generated Go bindings (auto-generated)
Makefile         # Build automation
```

### Implementing Tools

Tools are handled directly in the `HandleCallTool` function:

```go
func (m *MCPProvider) HandleCallTool(request cm.CallToolRequest) cm.Result[cm.ToolResult, cm.McpError, cm.McpError] {
    switch request.Name {
    case "echo":
        return handleEcho(request.Arguments)
    case "get_weather":
        return handleGetWeather(request.Arguments)
    case "multi_weather":
        return handleMultiWeather(request.Arguments)
    default:
        return cm.Err[cm.ToolResult, cm.McpError](cm.McpError{
            Code:    "tool_not_found",
            Message: fmt.Sprintf("Unknown tool: %s", request.Name),
        })
    }
}

func handleGetWeather(args cm.Option[string]) cm.Result[cm.ToolResult, cm.McpError, cm.McpError] {
    var params WeatherParams
    if argStr := args.Some(); argStr != nil {
        json.Unmarshal([]byte(*argStr), &params)
    }
    
    // Fetch weather data
    weather := getWeatherForCity(params.Location)
    return textResult(weather)
}
```

## Concurrency

Go's Wasm environment uses `wasihttp.GetConcurrently()` for concurrent HTTP operations. Example from the multi-weather implementation:

```go
func handleMultiWeather(args cm.Option[string]) cm.Result[cm.ToolResult, cm.McpError, cm.McpError] {
    var params MultiWeatherParams
    if argStr := args.Some(); argStr != nil {
        json.Unmarshal([]byte(*argStr), &params)
    }
    
    // Build URLs for concurrent requests
    urls := make([]string, len(params.Cities))
    for i, city := range params.Cities {
        urls[i] = buildGeocodingURL(city)
    }
    
    // Concurrent HTTP requests
    responses := wasihttp.GetConcurrently(urls)
    
    // Process responses
    results := processWeatherResponses(responses)
    return textResult(formatResults(results))
}
```

## Testing

```bash
make test-all        # Run all tests
make test-echo       # Test echo tool
make test-weather    # Test weather tool
make test-multi      # Test concurrent fetching
```

## Authorization

OAuth 2.0 authorization is optional and configured in the `GetAuthConfig` method:

```go
func (m *MCPProvider) GetAuthConfig() cm.Option[cm.ProviderAuthConfig] {
    // Return None to disable authorization
    return cm.None[cm.ProviderAuthConfig]()
    
    // Or enable OAuth 2.0 protection:
    // return cm.Some(cm.ProviderAuthConfig{
    //     ExpectedIssuer: "https://your-domain.authkit.app",
    //     ExpectedAudiences: []string{"client_id"},
    //     JwksUri: "https://your-domain.authkit.app/oauth2/jwks",
    //     Policy: cm.None[string](),      // Optional Rego policy
    //     PolicyData: cm.None[string](),  // Optional policy data
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