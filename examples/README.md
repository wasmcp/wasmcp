# WasmCP Examples

This directory contains working examples of MCP (Model Context Protocol) handlers as WebAssembly components.

## typescript-weather

A TypeScript MCP handler that demonstrates:
- Echo tool for basic testing
- Weather tool with async HTTP requests using fetch
- Works with both Spin and wasmtime runtimes

### Quick Start

```bash
cd typescript-weather

# Build and compose the component
make compose

# Run with Spin
spin up

# OR run with wasmtime
wasmtime serve -Scli composed.wasm
```

### Testing the Tools

Test the echo tool:
```bash
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "echo",
      "arguments": {"message": "Hello!"}
    }
  }'
```

Test the weather tool:
```bash
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "weather",
      "arguments": {"location": "San Francisco"}
    }
  }'
```

## How It Works

1. **Handler**: The TypeScript handler implements MCP tools using the `wasmcp` SDK
2. **Gateway**: The pre-built gateway component (`wasmcp-spin.wasm`) handles HTTP and runtime integration
3. **Composition**: `wac plug` combines the handler and gateway into a single component (`composed.wasm`)
4. **Runtime**: The composed component runs on any WASI-compliant runtime (Spin, wasmtime, etc.)

The workflow is completely automated - no manual intervention needed between template and running server!