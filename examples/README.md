# wasmcp examples

Complete, working examples of MCP (Model Context Protocol) servers running as WebAssembly components.

## Weather Examples

Parallel implementations of a weather MCP server in different languages:

### 🦀 [`weather/rust/`](./weather/rust)
A Rust MCP handler featuring:
- Echo tool for basic testing
- Weather tool with async HTTP requests
- Clean project structure with NO WIT files needed
- Uses `wasmcp@0.2.7` with proc macros

### 📘 [`weather/typescript/`](./weather/typescript)
A TypeScript MCP handler featuring:
- Echo tool for basic testing  
- Weather tool with async fetch API
- Clean project structure with WIT deps from npm
- Uses `wasmcp@0.1.11` npm package

### 🐹 [`weather/go/`](./weather/go)
A Go MCP handler featuring:
- Echo tool with typed structs
- Weather tool with HTTP requests
- Multi-weather tool demonstrating concurrent goroutines
- Idiomatic generics API with automatic JSON unmarshaling
- Uses `wasmcp/src/sdk/go@v0.2.5` with TinyGo

## Quick Start

All examples follow the same workflow:

```bash
# Enter an example directory
cd weather/rust  # or weather/typescript or weather/go

# Build and compose the component
make compose

# Run with Spin (recommended)
spin up

# OR run with wasmtime
wasmtime serve -S cli -S http composed.wasm
```

Then test the MCP server:

```bash
# List available tools
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "method": "tools/list", "id": 1}'

# Get weather for a location
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {
      "name": "weather",
      "arguments": {"location": "San Francisco"}
    }
  }'
```

## Key Features

All examples demonstrate:

1. **Zero Configuration**: Create from template and immediately have a working MCP server
2. **Real Async Operations**: Weather tool makes actual HTTP requests to external APIs
3. **Clean Project Structure**: No WIT files to manage (Rust uses proc macros, TypeScript/Go bundle them in SDK)
4. **Production Ready**: Optimized builds, proper error handling, comprehensive testing
5. **Runtime Flexibility**: Works with Spin, wasmtime, or any WASI-compliant runtime

## Creating Your Own

Start with the official templates:

```bash
# Install templates
spin templates install --git https://github.com/fastertools/wasmcp --upgrade

# Create new Rust project
spin new -t wasmcp-rust my-rust-mcp

# Create new TypeScript project  
spin new -t wasmcp-typescript my-ts-mcp

# Create new Go project
spin new -t wasmcp-go my-go-mcp
```

See each example's README for details on implementing your own tools.