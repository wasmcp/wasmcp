<div align="center">

# `wasmcp`

**MCP servers as WebAssembly components**

Run [Model Context Protocol](https://modelcontextprotocol.io) servers on [Spin](https://github.com/fermyon/spin), [Wasmtime](https://github.com/bytecodealliance/wasmtime), or any WASI runtime.

</div>

## Quick Start

```bash
# Install templates
spin templates install --git https://github.com/fastertools/wasmcp --upgrade

# Create MCP server
spin new -t wasmcp-rust my-weather-server --accept-defaults
cd my-weather-server

# Build handler and compose with gateway
make build
make compose

# Run with wasmtime (or spin up for Spin)
wasmtime serve -S cli -S http composed.wasm

# Test it
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "method": "tools/list", "id": 1}'
```

That's it. Zero configuration, no WIT files, runs on any WASI runtime.

## Features

- **Clean SDKs**: Rust with proc macros (no WIT files), TypeScript with npm package, Go with TinyGo support
- **Full Async**: Native async/await support for HTTP, database, and I/O operations  
- **Any Runtime**: Spin, Wasmtime, or any WASI-compatible WebAssembly runtime
- **Production Ready**: Optimized builds, proper error handling, comprehensive testing
- **Component Composition**: Modular architecture via WebAssembly Component Model

## Language Support

### Rust
```rust
use wasmcp::{mcp_handler, ToolHandler, AsyncToolHandler};
use serde_json::json;

// Simple sync tool
struct EchoTool;

impl ToolHandler for EchoTool {
    const NAME: &'static str = "echo";
    const DESCRIPTION: &'static str = "Echo a message back to the user";
    
    fn input_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "message": { "type": "string", "description": "Message to echo back" }
            },
            "required": ["message"]
        })
    }
    
    fn execute(args: serde_json::Value) -> Result<String, String> {
        Ok(format!("Echo: {}", args["message"]))
    }
}

// Async tool with real HTTP requests
impl AsyncToolHandler for WeatherTool {
    const NAME: &'static str = "weather";
    const DESCRIPTION: &'static str = "Get weather information for a location";
    
    async fn execute_async(args: serde_json::Value) -> Result<String, String> {
        use spin_sdk::http::{Request, send};
        
        // Geocode location
        let geocoding_url = format!("https://geocoding-api.open-meteo.com/v1/search?name={}", 
            args["location"]);
        let response = send(Request::get(&geocoding_url)).await?;
        
        // Parse and fetch weather...
        Ok(format!("Weather for {}: 22°C, Sunny", args["location"]))
    }
}

#[mcp_handler(tools(EchoTool, WeatherTool))]
mod handler {}
```

### TypeScript
```typescript
import { createTool, createHandler, z } from 'wasmcp';

// Simple tool with schema validation
const echoTool = createTool({
  name: 'echo',
  description: 'Echo a message back to the user',
  schema: z.object({
    message: z.string().describe('Message to echo back')
  }),
  execute: async (args) => {
    return `Echo: ${args.message}`;
  }
});

// Weather tool with async fetch
const weatherTool = createTool({
  name: 'weather',
  description: 'Get current weather for a location',
  schema: z.object({
    location: z.string().describe('City name')
  }),
  execute: async (args) => {
    // Geocode location
    const geocoding = await fetch(
      `https://geocoding-api.open-meteo.com/v1/search?name=${args.location}`
    );
    const location = await geocoding.json();
    
    // Get weather
    const weather = await fetch(
      `https://api.open-meteo.com/v1/forecast?latitude=${location.latitude}&longitude=${location.longitude}`
    );
    
    return `Weather in ${args.location}: 22°C, Sunny`;
  }
});

export const handler = createHandler({
  tools: [echoTool, weatherTool]
});
```

## Examples

See [`examples/`](./examples) for complete working servers:
- **[`rust-weather`](./examples/rust-weather)** - Rust with async HTTP weather API
- **[`typescript-weather`](./examples/typescript-weather)** - TypeScript with fetch API

Both implement the same tools and work identically from the client's perspective.

## Architecture

```
┌─────────────┐      HTTP/JSON-RPC      ┌──────────────┐
│ MCP Client  │◄────────────────────────►│wasmcp-server │
│  (Claude)   │                          │   (Server)   │
└─────────────┘                          └──────┬───────┘
                                                 │
                                         Component Model
                                                 │
                                          ┌──────▼───────┐
                                          │ Your Handler │
                                          │ (Rust/TS/JS) │
                                          └──────────────┘
```

The server handles HTTP and MCP protocol. You just implement tools.

## Templates

```bash
# Rust (no WIT files needed)
spin new -t wasmcp-rust my-rust-server

# TypeScript (WIT bundled in npm)  
spin new -t wasmcp-typescript my-ts-server

# JavaScript
spin new -t wasmcp-javascript my-js-server
```

## Development

```bash
# Prerequisites
cargo install cargo-component
npm install -g @bytecodealliance/jco

# Build everything
make build-all

# Run tests
make test-all

# See all commands
make help
```

## Repository Structure

```
wasmcp/
├── examples/              # Complete example servers
│   ├── rust-weather/     # Rust async weather server
│   └── typescript-weather/ # TypeScript weather server
├── src/
│   ├── components/
│   │   └── wasmcp-server/  # MCP server component
│   └── sdk/
│       ├── wasmcp-rust/  # Rust SDK (crates.io)
│       └── wasmcp-typescript/ # TypeScript SDK (npm)
├── templates/            # Spin templates
└── wit/                  # Component interfaces
```

## License

Apache-2.0