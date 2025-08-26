# go-weather

MCP server in Go.

## Prerequisites

- [TinyGo](https://tinygo.org/getting-started/install/)
- [wac](https://github.com/bytecodealliance/wac) 
- [Wasmtime](https://wasmtime.dev/) or [Spin](https://developer.fermyon.com/spin)

## Quick Start

```bash
# Build and run
make compose
make run-wasmtime

# Test  
make test-echo
```

## Structure

- `main.go` - MCP handler implementation
- `Makefile` - Build commands
- `spin.toml` - Spin configuration

## Adding Tools

Edit `main.go` to add new tools:

```go
func init() {
    mcp.Handle(func(h *mcp.Handler) {
        h.Tool("my-tool", "Description", mySchema(), myHandler)
    })
}
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