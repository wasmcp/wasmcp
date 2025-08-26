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

```go
package main

import (
    "encoding/json"
    "fmt"
    
    mcp "github.com/fastertools/wasmcp/src/sdk/wasmcp-go"
)

func init() {
    mcp.Handle(func(h *mcp.Handler) {
        // Register a simple tool
        h.Tool("echo", "Echo a message", mcp.Schema(`{
            "type": "object",
            "properties": {
                "message": {"type": "string"}
            },
            "required": ["message"]
        }`), echoHandler)
        
        // Register a tool that makes HTTP requests
        h.Tool("weather", "Get weather", mcp.Schema(`{
            "type": "object", 
            "properties": {
                "location": {"type": "string"}
            }
        }`), weatherHandler)
    })
}

func echoHandler(args json.RawMessage) (string, error) {
    var params struct {
        Message string `json:"message"`
    }
    if err := json.Unmarshal(args, &params); err != nil {
        return "", err
    }
    return fmt.Sprintf("Echo: %s", params.Message), nil
}

func weatherHandler(args json.RawMessage) (string, error) {
    var params struct {
        Location string `json:"location"`
    }
    json.Unmarshal(args, &params)
    
    // Use the built-in HTTP client
    resp, err := mcp.DefaultHTTPClient.Get(fmt.Sprintf("https://api.weather.com/%s", params.Location))
    if err != nil {
        return "", err
    }
    
    return resp, nil
}

func main() {} // Required for TinyGo
```

## Building

```bash
# Build your handler (with wasip2 target)
tinygo build -target=wasip2-mcp.json -gc=leaking -no-debug -o handler.component.wasm main.go

# Compose with server (requires wac)
wac plug --plug handler.component.wasm wasmcp-server.wasm -o composed.wasm

# Run with wasmtime
wasmtime serve -S cli -S http composed.wasm
```

## API

### `mcp.Handle(func(*Handler))`

Register your MCP handler. Must be called in an `init()` function.

### `Handler.Tool(name, description string, schema json.RawMessage, fn ToolFunc)`

Register a tool that can be called by MCP clients.

### `Handler.Resource(uri, name, description, mimeType string, fn ResourceFunc)`

Register a resource that can be read by MCP clients.

### `Handler.Prompt(name, description string, arguments []PromptArgument, fn PromptFunc)`

Register a prompt template.

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

- Requires TinyGo for compilation to WebAssembly
- Uses wit-bindgen-go for WIT bindings (generated)
- Compatible with any WASI runtime (Spin, Wasmtime, etc.)