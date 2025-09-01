# {{project-name}}

{{project-description}}

## Quick Start

```bash
make setup  # Install dependencies and verify environment
make build  # Build the MCP server
make serve  # Run the server (default: wasmtime on port 8080)
```

Test the server:
```bash
make test-all  # Run all tests
```

## Architecture

This MCP server runs as a WebAssembly component, combining:
- **Provider**: Your Go implementation of MCP tools (this code)
- **Transport**: Pre-built HTTP server component from the registry

The composition happens at build time, producing a single `mcp-http-server.wasm` that can run on any runtime that supports the Wasm component model.

## Development

### Prerequisites

- **Go 1.23+** and **TinyGo 0.34+** - Required for WASM compilation
- **wasm-tools** - Component model toolchain
- **wit-bindgen-go** - Generate Go bindings from WIT interfaces

Quick setup:
```bash
make setup  # Checks and installs all dependencies
```

### Project Structure

```
├── main.go          # Tool implementations
├── helpers.go       # MCP SDK-like helper functions
├── wasihttp/        # WASI HTTP client with concurrent support
├── wit/             # WebAssembly Interface Types
├── internal/        # Generated bindings (don't edit)
└── Makefile         # Build automation
```

### Build Pipeline

The build process has three stages:

```bash
make bindgen         # Generate Go bindings from WIT
make build-provider  # Compile Go to WASM component
make build          # Compose with transport
```

Or simply: `make build` (runs all steps)

### Adding New Tools

To add a new tool, register it in the `init()` function:

```go
AddTool(server, &Tool{
    Name:        "my_tool",
    Description: "Tool description",
    InputSchema: Schema(`{"type": "object", ...}`),
}, handleMyTool)
```

Then implement the handler function:

```go
func handleMyTool(ctx context.Context, args MyToolArgs) (*CallToolResult, error) {
    // Implementation
    return TextResult("result"), nil
}
```

## Concurrency in TinyGo/WASM

TinyGo runs with `GOMAXPROCS=1` and uses cooperative scheduling, meaning goroutines execute sequentially on a single thread. Traditional Go concurrency patterns won't provide parallel execution.

For concurrent HTTP requests, this template includes `wasihttp.GetConcurrently()` which leverages WASI's native polling mechanism. Instead of blocking on each request sequentially, it:

1. Starts all HTTP requests without waiting for responses
2. Uses WASI's `poll.Poll()` to efficiently wait for any request to complete
3. Processes responses as they arrive from the host

This achieves true concurrent I/O by delegating to the WASI host's async capabilities, bypassing TinyGo's single-threaded limitation. See the `multi_weather` tool for an example.

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

## Debugging

### Common Issues

**Build fails with "undefined: cm.LiftOption"**
- TinyGo doesn't support all Go reflection features. Use `.Some()` to access Option values.

**HTTP requests timeout or fail**
- Ensure `http.DefaultTransport = &wasihttp.Transport{}` is set in `init()`
- Check that URLs are accessible from your environment

**Server doesn't start**
- Verify port 8080 is available: `lsof -i :8080`
- Check wasmtime is installed: `which wasmtime`

### Inspecting the Component

```bash
make inspect  # Show component structure and exports
```

## Runtime Options

The server can run on any WASI-compliant runtime:

```bash
# Wasmtime (default)
wasmtime serve -Scli ./mcp-http-server.wasm

# Spin
spin up
```

