# wasmcp-spin

HTTP gateway component for MCP servers. Handles protocol translation between HTTP/JSON-RPC and WebAssembly components.

## What It Does

Bridges HTTP requests to your MCP handler:
- Receives JSON-RPC requests at `/mcp` endpoint  
- Translates to component calls
- Returns JSON-RPC responses
- Handles errors gracefully

## Usage

Most users get this automatically from templates. It's included in `spin.toml`:

```toml
[component.wasmcp-spin]
source = "wasmcp-spin.wasm"

[component.wasmcp-spin.dependencies]
"wasmcp:mcp/handler@0.1.0" = { path = "./handler.wasm" }
```

## Protocol Support

- **MCP methods**: `tools/list`, `tools/call`, `resources/list`, `resources/read`, `prompts/list`, `prompts/get`
- **JSON-RPC 2.0**: Full spec compliance with batching
- **Error codes**: Standard MCP error codes (-32601, -32603, etc.)

## Building Custom Gateway

```bash
cargo component build --release --target wasm32-wasip2
```

## Architecture

```
HTTP Request → Spin Trigger → wasmcp-spin → Your Handler
     ↓             ↓              ↓              ↓
 JSON-RPC    IncomingRequest  Component    Tool/Resource
              /Response         Call        Implementation
```

The gateway is stateless - all state lives in your handler or Spin's KV stores.

## License

Apache-2.0