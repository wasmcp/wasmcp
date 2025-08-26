# go-weather-v2

An MCP (Model Context Protocol) server written in Go.

## Prerequisites

- [TinyGo](https://tinygo.org/getting-started/install/)
- [wasm-tools](https://github.com/bytecodealliance/wasm-tools)
- [wac](https://github.com/bytecodealliance/wac)
- [Spin](https://developer.fermyon.com/spin/v2/install) (optional, for `spin up`)
- [Wasmtime](https://wasmtime.dev/) (optional, for standalone execution)

## Quick Start

```bash
# Build and compose the component
make compose

# Run with wasmtime
make run-wasmtime

# Or run with Spin
make run

# Test the echo tool
make test-echo
```

## Project Structure

- `main.go` - Your MCP handler implementation
- `Makefile` - Build commands
- `spin.toml` - Spin configuration
- `wasmcp-server.wasm` - Pre-built MCP server component

## Adding Tools

Edit `main.go` to add new tools:

```go
func init() {
    mcp.Handle(func(h *mcp.Handler) {
        h.Tool("my-tool", "Description", mySchema(), myHandler)
    })
}
```

## Using Spin SDK Features

To use HTTP, Key-Value, or other Spin features, add the Spin SDK to your `go.mod`:

```bash
go get github.com/fermyon/spin-go-sdk/v2
```

Then import what you need:

```go
import spinhttp "github.com/fermyon/spin-go-sdk/v2/http"
```

## Testing

Test your tools with curl:

```bash
# List available tools
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":1}'

# Call a tool
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":{"message":"test"}},"id":1}'
```

## License

Apache-2.0