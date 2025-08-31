# {{project-name}}

{{project-description}}

## Quick Start

```bash
# Build the MCP server
make build

# Run the server with Spin
spin up

# Or use the Makefile target
make run
```

## Testing

Test the MCP server with the built-in test commands:

```bash
# Test all endpoints
make test-all

# Test individual tools
make test-echo
make test-weather
make test-multi
```

## Development

### Prerequisites

- Go 1.23+ and TinyGo
- wasm-tools
- wkg (WebAssembly Component Registry tools)
- wac (WebAssembly Compositions)
- wit-bindgen-go

Run the setup script to check and install dependencies:

```bash
make setup
```

### Build Process

1. **Generate bindings**: `make bindgen` - Creates Go bindings from WIT definitions
2. **Build provider**: `make build-provider` - Compiles Go code to WASM component
3. **Compose server**: `make build` - Combines provider with MCP transport

### Project Structure

- `main.go` - Main implementation file with MCP tools
- `wit/` - WebAssembly Interface Types definitions
- `internal/` - Generated Go bindings (auto-generated, do not edit)
- `Makefile` - Build and test automation

### Adding New Tools

1. Add the tool definition in `handleListTools` function
2. Implement the handler function
3. Add a case in `handleCallTool` switch statement
4. Update test targets in Makefile if needed

## Concurrent HTTP Support

This implementation uses TinyGo's asyncify scheduler to support goroutines for concurrent HTTP requests. The `multi_weather` tool demonstrates concurrent fetching using `sync.WaitGroup`.

