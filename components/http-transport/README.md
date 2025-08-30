# mcp-http-transport

Feature-flagged MCP HTTP transport component that handles HTTP and JSON-RPC protocol. Works with any WASI runtime (Spin, Wasmtime, etc.).

## What It Does

Bridges HTTP requests to your MCP capability provider:
- Receives JSON-RPC requests
- Translates to component calls
- Returns JSON-RPC responses
- Handles errors gracefully
- **Build variants that only import the capabilities they need (no null components!)**

## Why Multiple Transport Variants?

The WebAssembly Component Model requires that all imports declared by a component must be satisfied during composition. This creates a challenge: if the transport imports all MCP capabilities (tools, resources, prompts, etc.), then EVERY provider would need to export ALL interfaces, even ones it doesn't use.

The solution is to build different transport variants with different import requirements. This allows providers to export only the capabilities they actually implement, resulting in cleaner provider code and clearer capability contracts.

## How Feature-Based Compilation Works

The transport uses a clever build system to create different transport variants:

1. **Build Script (`build.rs`)**: Automatically selects the right WIT interface based on enabled Cargo features
2. **WIT Variants (`wit-variants/`)**: Pre-defined world definitions for each transport variant
3. **Feature Flags**: Cargo features control which MCP capabilities are compiled in

This approach works around a fundamental component model constraint while maintaining good developer experience

### Available Variants

| Variant | Features | Use Case |
|---------|----------|----------|
| `tools-transport` | Tools only | Providers that just provide tools (e.g., weather, calculators) |
| `resources-transport` | Resources only | Providers that just serve resources (e.g., file systems) |
| `prompts-transport` | Prompts only | Providers that just provide prompts |
| `tools-resources-transport` | Tools + Resources | Most common combination |
| `tools-resources-prompts-transport` | Tools + Resources + Prompts | Full-featured providers |
| `full-transport` | All features | Everything including future capabilities |

## Building Transport Variants

### Quick Build (All Variants)
```bash
# Builds all 6 variants and saves them to target/
./build-variants.sh
```

### Build Specific Variant
```bash
# The build.rs automatically copies the right WIT file based on features
cargo component build --features "tools" --release
# Creates: target/wasm32-wasip1/release/mcp_transport_http.wasm (tools-only variant)

cargo component build --features "tools,resources,prompts" --release  
# Creates: target/wasm32-wasip1/release/mcp_transport_http.wasm (standard variant)
```

## Usage

### Usage Example
```wac
// Use a transport variant that matches your provider's capabilities
let provider = new my:provider { ... };

// For a tools-only provider, use tools-transport variant
let transport = new fastertools:mcp-http-tools-server {
    "fastertools:mcp/tools-capabilities@0.1.10": provider["fastertools:mcp/tools-capabilities@0.1.10"],
    ...
};

// For a provider with tools and resources, use tools-resources-transport variant
let transport = new fastertools:mcp-http-tools-resources-server {
    "fastertools:mcp/tools-capabilities@0.1.10": provider["fastertools:mcp/tools-capabilities@0.1.10"],
    "fastertools:mcp/resources-capabilities@0.1.10": provider["fastertools:mcp/resources-capabilities@0.1.10"],
    ...
};

export transport["wasi:http/incoming-handler@0.2.0"];
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
HTTP Request → WASI Runtime → mcp-transport → Your Provider
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
   - Transport receives HTTP requests with JSON-RPC payloads
   - Routes methods based on compiled features (using `#[cfg(feature = "...")]`)
   - Only calls capabilities that were imported at compile time
   - Returns proper MCP error if method not available in this variant

3. **Composition Time**:
   - Provider only needs to export the capabilities it implements
   - Transport variant only imports what it needs
   - No null components required!

The transport is stateless - all state lives in your provider or external stores.

## License

Apache-2.0