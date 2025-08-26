# wasmcp-go

Go SDK for building MCP (Model Context Protocol) handlers as WebAssembly components.

## Installation

```bash
go get github.com/fastertools/wasmcp/src/sdk/wasmcp-go
```

```go
import mcp "github.com/fastertools/wasmcp/src/sdk/wasmcp-go"
```

## Usage

### Basic Example

```go
package main

import (
    "context"
    mcp "github.com/fastertools/wasmcp/src/sdk/wasmcp-go"
)

func init() {
    server := mcp.NewServer(
        &mcp.Implementation{Name: "my-server", Version: "v1.0.0"},
        nil,
    )
    
    // Define argument struct
    type EchoArgs struct {
        Message string `json:"message"`
    }
    
    // Add tool with typed handler - automatic JSON unmarshaling
    mcp.AddTool(server, &mcp.Tool{
        Name:        "echo",
        Description: "Echo a message",
        InputSchema: mcp.Schema(`{
            "type": "object",
            "properties": {
                "message": {"type": "string", "description": "Message to echo"}
            },
            "required": ["message"]
        }`),
    }, func(ctx context.Context, args EchoArgs) (*mcp.CallToolResult, error) {
        return &mcp.CallToolResult{
            Content: []mcp.Content{
                &mcp.TextContent{Text: "Echo: " + args.Message},
            },
        }, nil
    })
    
    server.Run(context.Background(), nil)
}

func main() {} // Required for TinyGo
```

### Schema Generation

Since TinyGo has limited reflection support, schemas must be defined manually or generated at build time. We recommend:

1. **Manual schemas** - Use `mcp.Schema()` with JSON strings (shown above)
2. **Build-time generation** - Use a code generator (coming soon):
   ```bash
   # Future feature
   go run github.com/fastertools/wasmcp/cmd/mcp-gen ./...
   ```
   This will generate schemas from struct tags:
   ```go
   type WeatherArgs struct {
       Location string `json:"location" mcp:"required,description:City name"`
       Units    string `json:"units" mcp:"enum:celsius|fahrenheit,default:celsius"`
   }
   ```

## Building

```bash
# Build your handler
tinygo build -target=wasip2 -scheduler=asyncify -no-debug -o handler.wasm main.go

# Compose with server
wac plug --plug handler.wasm wasmcp-server.wasm -o composed.wasm

# Run
wasmtime serve -S cli -S http composed.wasm
```

## API

### `mcp.NewServer(impl *Implementation, opts *ServerOptions) *Server`

Create a new MCP server instance.

### `mcp.AddTool[In any](server *Server, tool *Tool, handler func(context.Context, In) (*CallToolResult, error))`

Register a tool with typed handler. The generic type `In` defines the argument structure and the SDK automatically handles JSON unmarshaling.

### `server.AddResource(resource *Resource, handler func(context.Context) (string, error))`

Register a resource that can be read by MCP clients.

### `mcp.AddPrompt[In any](server *Server, prompt *Prompt, handler func(context.Context, In) ([]PromptMessage, error))`

Register a prompt template with typed arguments.

### `mcp.Schema(string) json.RawMessage`

Helper to create JSON schema definitions inline.

## HTTP Support

The SDK automatically enables WASI HTTP support, so you can use Go's standard `net/http` package directly:

```go
// Standard net/http just works!
resp, err := http.Get("https://api.example.com/data")

// Concurrent requests with goroutines work too
var wg sync.WaitGroup
for _, url := range urls {
    wg.Add(1)
    go func(u string) {
        defer wg.Done()
        resp, _ := http.Get(u)
        // Process response
    }(url)
}
wg.Wait()
```

## Notes

- Requires TinyGo with wasip2 support
- Compatible with any wasip2 runtime (Wasmtime, Spin, etc.)
- Pure WebAssembly Component Model - no adapters needed