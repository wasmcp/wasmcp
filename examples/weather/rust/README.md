# Rust MCP Server

WebAssembly Component Model implementation of MCP (Model Context Protocol) in Rust, featuring async HTTP with concurrent requests.

## Workflow

```bash
# Install template (one-time setup)
spin templates install --git https://github.com/ianpurton/wasmcp

# Create new MCP server
spin new -t wasmcp-rust my-mcp-server
cd my-mcp-server

# Build and run locally
make build              # Creates composed.wasm
wasmtime serve -Scli composed.wasm  # Direct WASI execution, no framework needed!
# OR
make run                # Same thing via make
# OR  
spin up                 # Using Spin platform features

# Test it works
make test-tools         # List available tools

# Deploy anywhere WASI runs
spin deploy             # To Spin Cloud
# OR copy composed.wasm to any WASI host
```


## Architecture

The build process uses simple WebAssembly Component Model plugging:

```
Handler Component → Server Variant = composed.wasm
     (your code)     (HTTP server)    (standard WASI)
```

No composition file needed! The build process:
1. Downloads the server variant from the registry (`wkg`)
2. Plugs your handler into it (`wac plug`)
3. Creates a single deployable WASI component

The resulting `composed.wasm` is a **pure WASI component** - no special runtime required:

```bash
# Direct execution with wasmtime - no frameworks, no adapters!
wasmtime serve -Scli composed.wasm
```

This works because it's standard WASI HTTP - runs anywhere:
- **Wasmtime**: Direct execution, no modifications needed
- **Spin**: Additional platform features (KV, deploy, etc.)
- **Any WASI runtime**: Docker+Wasm, WasmEdge, Wasmer, Fastly, etc.

## Key Files

```
├── src/
│   ├── lib.rs       # Your MCP tools implementation
│   └── helpers.rs   # Minimal async trait helpers (~140 lines)
├── wit/world.wit    # WIT interface declarations
├── Makefile         # Build automation (using wac plug)
└── composed.wasm    # Final deployable component (after build)
```

## Development

```bash
# Core commands
make build          # Build everything into composed.wasm
make run            # Build and serve locally
make serve          # Serve existing composed.wasm
make test-tools     # Test tool endpoints

# Additional targets
make build-handler  # Build just your component (quick compile check)
make clean          # Clean all artifacts
```

## Implementation

The template provides a clean async-first design with minimal boilerplate:

```rust
use helpers::{Tool, ToolResult, McpError, text_result};

struct WeatherTool;

impl Tool for WeatherTool {
    const NAME: &'static str = "get_weather";
    const DESCRIPTION: &'static str = "Get current weather";
    
    async fn execute(args: Value) -> Result<ToolResult, McpError> {
        // Real async HTTP - concurrent requests supported
        let weather = get_weather_for_city(location).await?;
        Ok(text_result(weather))
    }
}
```

### Key Features

- **Async by default**: Single `Tool` trait with async execution
- **Real concurrency**: Multiple HTTP requests run in parallel within the component
- **Clean imports**: Re-exported types avoid long binding paths
- **No complex SDKs**: Just ~140 lines of helper traits in your project
- **Zero Send issues**: Associated functions pattern avoids Rust's Send trait complications

### Extending Capabilities

To add resources or prompts:
1. Export the interface in `wit/world.wit`
2. Implement the trait in `src/lib.rs`
3. Use a different server variant (e.g., `wasmcp-server-basic` for tools+resources)

## Testing

```bash
make test-init      # Test initialization
make test-tools     # List available tools

# Call a tool
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_weather","arguments":{"location":"London"}},"id":1}'
```

## Deployment

The `composed.wasm` is a standard WASI component that runs on any compliant runtime. 

Try `spin deploy` to to run on [Fermyon Cloud](https://developer.fermyon.com/cloud/index)

## Technical Details

- **Composition**: Simple `wac plug` connects handler directly to server variant
- **Async Runtime**: `spin_sdk::http::run()` provides WASI-compatible async execution
- **Concurrency**: Real parallel HTTP via `futures::join_all` within component boundaries
- **Size**: ~853KB composed (handler ~347KB, server ~506KB)

## Learn More

- [MCP Specification](https://modelcontextprotocol.io)
- [WebAssembly Component Model](https://component-model.bytecodealliance.org/)
- [Spin Documentation](https://developer.fermyon.com/spin)