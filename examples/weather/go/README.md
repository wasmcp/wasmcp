# go-weather

MCP weather server in Go with concurrent goroutines.

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
make test-weather
```

## Features

This example demonstrates:
- **Echo tool** - Simple typed struct handler
- **Weather tool** - Real HTTP requests to Open-Meteo API
- **Multi-weather tool** - Concurrent requests using goroutines
- **Idiomatic Go API** - Generic functions with automatic JSON unmarshaling

## Structure

- `main.go` - MCP handler implementation
- `Makefile` - Build commands
- `spin.toml` - Spin configuration

## Adding Tools

Edit `main.go` to add new tools:

```go
func init() {
    server := mcp.NewServer(
        &mcp.Implementation{Name: "weather", Version: "v1.0.0"},
        nil,
    )
    
    // Define your argument struct
    type MyArgs struct {
        Input string `json:"input"`
    }
    
    // Add tool with typed handler
    mcp.AddTool(server, &mcp.Tool{
        Name:        "my-tool",
        Description: "Description",
        InputSchema: mcp.Schema(`{...}`),
    }, func(ctx context.Context, args MyArgs) (*mcp.CallToolResult, error) {
        // Your implementation
        return &mcp.CallToolResult{
            Content: []mcp.Content{
                &mcp.TextContent{Text: result},
            },
        }, nil
    })
}
```


## Testing

Test your tools with curl:

```bash
# List available tools
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":1}'

# Call echo tool
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":{"message":"test"}},"id":1}'

# Get weather for multiple cities concurrently
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"multi_weather","arguments":{"cities":["Tokyo","Paris","New York"]}},"id":1}'
```

## License

Apache-2.0