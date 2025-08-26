# wasmcp-go

Go SDK for building MCP (Model Context Protocol) handlers as WebAssembly components.

## Installation

```go
import mcp "github.com/fastertools/wasmcp-go"
```

## Usage

```go
package main

import (
    "encoding/json"
    "fmt"
    
    mcp "github.com/fastertools/wasmcp-go"
    spinhttp "github.com/fermyon/spin-go-sdk/http"  // Use Spin SDK directly for HTTP
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
    
    // Use Spin SDK directly for HTTP requests
    resp, err := spinhttp.Get(fmt.Sprintf("https://api.weather.com/%s", params.Location))
    if err != nil {
        return "", err
    }
    
    return resp.Body, nil
}

func main() {} // Required for TinyGo
```

## Building

```bash
# Build your handler
tinygo build -target=wasip1 -gc=leaking -buildmode=c-shared -no-debug -o handler.wasm .

# Convert to component (requires wasm-tools)
wasm-tools component new handler.wasm --adapt wasi_snapshot_preview1.reactor.wasm -o handler.component.wasm

# Compose with server (requires wac)
wac plug handler.component.wasm wasmcp-server.wasm -o composed.wasm
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

## Integration with Spin SDK

This SDK focuses only on MCP functionality. For additional capabilities, import the Spin SDK directly:

- HTTP client: `github.com/fermyon/spin-go-sdk/http`
- Key-Value store: `github.com/fermyon/spin-go-sdk/kv`
- SQLite: `github.com/fermyon/spin-go-sdk/sqlite`
- Variables: `github.com/fermyon/spin-go-sdk/variables`

## Notes

- Requires TinyGo for compilation to WebAssembly
- Uses CGO for WIT bindings (included)
- Compatible with any WASI runtime (Spin, Wasmtime, etc.)