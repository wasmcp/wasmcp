# wasmcp-server

Feature-flagged MCP server component that handles HTTP and JSON-RPC protocol. Works with any WASI runtime (Spin, Wasmtime, etc.).

## What It Does

Bridges HTTP requests to your MCP handler:
- Receives JSON-RPC requests
- Translates to component calls
- Returns JSON-RPC responses
- Handles errors gracefully
- **NEW**: Build variants that only import the handlers they need (no null components!)

## Building Server Variants

### Quick Build
```bash
# Build all common variants
./build-variants.sh
```

### Manual Build
```bash
# Tools-only server (for handlers that only provide tools)
cp wit-variants/server-tools.wit wit/world.wit
cargo component build --features "tools" --no-default-features --release

# Standard server (tools + resources + prompts)
cp wit-variants/server-standard.wit wit/world.wit
cargo component build --features "tools,resources,prompts" --no-default-features --release
```

## Usage

### New Approach - Use Matching Server Variant
```wac
// Use a server variant that matches your handler's capabilities
let handler = new my:handler { ... };

// Tools-only server - no null components needed!
let server = new fastertools:wasmcp-server-tools {
    "fastertools:mcp/tool-handler@0.1.3": handler["fastertools:mcp/tool-handler@0.1.3"],
    ...
};

export server["wasi:http/incoming-handler@0.2.0"];
```

### Legacy Approach - With Null Components
```wac
// Old way required null components for unused capabilities
let handler = new my:handler { ... };
let nullresources = new fastertools:null-resources { ... };
let nullprompts = new fastertools:null-prompts { ... };

let server = new fastertools:wasmcp-server {
    "fastertools:mcp/tool-handler@0.1.3": handler["fastertools:mcp/tool-handler@0.1.3"],
    "fastertools:mcp/resource-handler@0.1.3": nullresources["fastertools:mcp/resource-handler@0.1.3"],
    "fastertools:mcp/prompt-handler@0.1.3": nullprompts["fastertools:mcp/prompt-handler@0.1.3"],
    ...
};
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