# wasmcp-server

Feature-flagged MCP server component that handles HTTP and JSON-RPC protocol. Works with any WASI runtime (Spin, Wasmtime, etc.).

## What It Does

Bridges HTTP requests to your MCP handler:
- Receives JSON-RPC requests
- Translates to component calls
- Returns JSON-RPC responses
- Handles errors gracefully
- **Build variants that only import the handlers they need (no null components!)**

## How Feature-Based Compilation Works

The server uses a clever build system to create different server variants:

1. **Build Script (`build.rs`)**: Automatically selects the right WIT interface based on enabled Cargo features
2. **WIT Variants (`wit-variants/`)**: Pre-defined world definitions for each server variant
3. **Feature Flags**: Cargo features control which MCP capabilities are compiled in

### Available Variants

| Variant | Features | Use Case |
|---------|----------|----------|
| `server-tools` | Tools only | Handlers that just provide tools (e.g., weather, calculators) |
| `server-resources` | Resources only | Handlers that just serve resources (e.g., file systems) |
| `server-prompts` | Prompts only | Handlers that just provide prompts |
| `server-basic` | Tools + Resources | Most common combination |
| `server-standard` | Tools + Resources + Prompts | Full-featured handlers |
| `server-full` | All features | Everything including future capabilities |

## Building Server Variants

### Quick Build (All Variants)
```bash
# Builds all 6 variants and saves them to target/
./build-variants.sh
```

### Build Specific Variant
```bash
# The build.rs automatically copies the right WIT file based on features
cargo component build --features "tools" --release
# Creates: target/wasm32-wasip1/release/wasmcp_server.wasm (tools-only variant)

cargo component build --features "tools,resources,prompts" --release  
# Creates: target/wasm32-wasip1/release/wasmcp_server.wasm (standard variant)
```

## Usage

### Usage Example
```wac
// Use a server variant that matches your handler's capabilities
let handler = new my:handler { ... };

// For a tools-only handler, use server-tools variant
let server = new fastertools:wasmcp-server-tools {
    "fastertools:mcp/tool-handler@0.1.4": handler["fastertools:mcp/tool-handler@0.1.4"],
    ...
};

// For a handler with tools and resources, use server-basic variant
let server = new fastertools:wasmcp-server-basic {
    "fastertools:mcp/tool-handler@0.1.4": handler["fastertools:mcp/tool-handler@0.1.4"],
    "fastertools:mcp/resource-handler@0.1.4": handler["fastertools:mcp/resource-handler@0.1.4"],
    ...
};

export server["wasi:http/incoming-handler@0.2.0"];
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

### How It Works Under the Hood

1. **Compile Time**: 
   - `build.rs` runs before compilation
   - Checks enabled Cargo features (`CARGO_FEATURE_*` env vars)
   - Copies appropriate WIT file from `wit-variants/` to `wit/world.wit`
   - cargo-component generates bindings from the selected WIT

2. **Runtime**:
   - Server receives HTTP requests with JSON-RPC payloads
   - Routes methods based on compiled features (using `#[cfg(feature = "...")]`)
   - Only calls handlers that were imported at compile time
   - Returns proper MCP error if method not available in this variant

3. **Composition Time**:
   - Handler only needs to export the capabilities it implements
   - Server variant only imports what it needs
   - No null components required!

The server is stateless - all state lives in your handler or external stores.

## License

Apache-2.0