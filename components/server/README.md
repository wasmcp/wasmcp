# wasmcp-server

MCP server component that handles HTTP and JSON-RPC protocol. Works with any WASI runtime (Spin, Wasmtime, etc.).

## What It Does

Bridges HTTP requests to your MCP handler:
- Receives JSON-RPC requests
- Translates to component calls
- Returns JSON-RPC responses
- Handles errors gracefully

## Usage

Compose with your handler using `wac`:

```bash
wac plug --plug handler.wasm wasmcp-server.wasm -o composed.wasm
```

Run with any WASI runtime:

```bash
# Wasmtime
wasmtime serve -S cli -S http composed.wasm

# Spin
spin up
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
HTTP Request → WASI Runtime → wasmcp-server → Your Handler
     ↓             ↓              ↓              ↓
 JSON-RPC    IncomingRequest  Component    Tool/Resource
              /Response         Call        Implementation
```

The server is stateless - all state lives in your handler or external stores.

## License

Apache-2.0