# Go MCP Weather Provider

A Model Context Protocol (MCP) provider written in Go using TinyGo and the WebAssembly Component Model.

This example demonstrates:
- Building MCP providers in Go with TinyGo
- WebAssembly Component Model integration via `wit-bindgen-go`
- Concurrent HTTP requests using Go's native concurrency
- Composition with transport components

## Prerequisites

- Go 1.24+ (for `go tool` directive support)
- TinyGo 0.34.0+ (WebAssembly Component Model support)
- wasm-tools (component tooling)
- wkg (WIT package management)
- wac (component composition)

## Quick Start

1. **Run setup to check dependencies:**
```bash
make setup
```

2. **Build the component:**
```bash
make build
```

This creates `mcp-http-server.wasm` by:
- Generating Go bindings from WIT with `wit-bindgen-go`
- Compiling Go to WASM with TinyGo's `wasip2` target
- Composing the provider with an HTTP transport

3. **Run the server:**
```bash
make run
# Starts on http://localhost:8080
```

4. **Test the tools:**
```bash
# List available tools
make test-tools

# Test individual tools
make test-echo
make test-weather
make test-multi
```

## Architecture

### Go → WASM Compilation

TinyGo compiles Go code to WebAssembly with Component Model support:

```go
//go:generate go tool wit-bindgen-go generate --world weather-go --out internal ./wit

func init() {
    // Register MCP capability implementations
    toolscapabilities.Exports.HandleListTools = handleListTools
    toolscapabilities.Exports.HandleCallTool = handleCallTool
}
```

### WIT Interface

The `weather-go` world defines the component's exports:

```wit
world weather-go {
    include wasi:cli/imports@0.2.0;  // Required for TinyGo wasip2
    export fastertools:mcp/tools-capabilities@0.1.10;
    import wasi:http/outgoing-handler@0.2.0;  // For HTTP requests
}
```

### Concurrent Operations

Go's goroutines enable natural concurrent patterns:

```go
func fetchMultiWeather(ctx context.Context, cities []string) []weatherResult {
    results := make([]weatherResult, len(cities))
    var wg sync.WaitGroup
    
    for i, city := range cities {
        wg.Add(1)
        go func(idx int, c string) {
            defer wg.Done()
            data, err := fetchWeather(ctx, c)
            results[idx] = weatherResult{data: data, err: err}
        }(i, city)
    }
    
    wg.Wait()
    return results
}
```

## Available Tools

### echo
Echoes a message back to the user.

```json
{
  "name": "echo",
  "arguments": {"message": "Hello from Go!"}
}
```

### get_weather
Fetches current weather for a single location.

```json
{
  "name": "get_weather",
  "arguments": {"location": "Tokyo"}
}
```

### multi_weather
Fetches weather for multiple cities concurrently.

```json
{
  "name": "multi_weather",
  "arguments": {"cities": ["Tokyo", "London", "New York"]}
}
```

## Build Options

### Standard Build
```bash
make build
```
Includes debug information for development.

### Optimized Build
```bash
make build-optimized
```
Removes debug info, reducing size by up to 75%.

### Clean Build
```bash
make clean      # Remove build artifacts
make clean-all  # Also clean Go cache
```

## Development

### Generate Bindings
```bash
make bindgen
```
Regenerates Go bindings from WIT definitions.

### Inspect Component
```bash
make inspect
```
Shows the component's exported interfaces.

### WIT Package Management
```bash
make wit-deps   # Fetch WIT dependencies
make wit-build  # Build WIT package
```

## Deployment

The built `mcp-http-server.wasm` runs on any WebAssembly Component Model runtime:

### Wasmtime
```bash
wasmtime serve -Scli mcp-http-server.wasm
```

### Spin
```bash
spin up --from mcp-http-server.wasm
```

### Claude Desktop
```json
{
  "mcpServers": {
    "weather-go": {
      "command": "wasmtime",
      "args": ["serve", "-Scli", "/path/to/mcp-http-server.wasm"],
      "transport": "http",
      "url": "http://localhost:8080/mcp"
    }
  }
}
```

## Troubleshooting

### TinyGo Version
Ensure TinyGo 0.34.0+ for Component Model support:
```bash
tinygo version
```

### Go Version
Go 1.24+ is recommended for `go tool` directive:
```bash
go version
```

### Missing Imports
If WIT imports are missing:
```bash
wkg wit fetch
make wit-build
```

### Build Errors
Clean and rebuild:
```bash
make clean
make deps
make build
```

## Performance

- **Debug builds**: Include symbols and debug info (~10-15MB)
- **Optimized builds**: Strip debug info (~2-3MB)
- **Concurrent requests**: Leverage Go's goroutines for parallel operations
- **Memory usage**: TinyGo optimizes for small memory footprint

## References

- [TinyGo Documentation](https://tinygo.org/docs/)
- [Component Model Go Support](https://component-model.bytecodealliance.org/language-support/go.html)
- [wit-bindgen-go](https://github.com/bytecodealliance/go-modules)
- [MCP Specification](https://modelcontextprotocol.io/)