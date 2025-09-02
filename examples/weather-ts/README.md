# weather-ts

An MCP server written in TypeScript

## Quick Start

```bash
make setup  # Install dependencies and verify environment
make build  # Build the MCP server
make serve  # Run the server (default: wasmtime on port 8080)
```

Test the server:
```bash
make test-all  # Run all tests
```

## Architecture

This MCP server runs as a WebAssembly component, combining:
- **Provider**: Your TypeScript implementation of MCP tools (this code)
- **Transport**: Pre-built HTTP server component from the registry

The composition happens at build time, producing a single `mcp-http-server.wasm` that can run on any runtime that supports the Wasm component model.

## Development

### Prerequisites

- **Node.js 20+** - Required for TypeScript and jco
- **jco** - Compiles JavaScript/TypeScript to Wasm components
- **wasm-tools** - Component model toolchain

Quick setup:
```bash
make setup  # Checks and installs all dependencies
```

### Project Structure

```
├── src/
│   ├── index.ts     # Tool implementations
│   ├── helpers.ts   # MCP SDK-like helper functions
│   └── types.ts     # TypeScript type definitions
├── wit/             # WebAssembly Interface Types
├── dist/            # Compiled JavaScript (generated)
└── Makefile         # Build automation
```

### Build Pipeline

The build process has four stages:

```bash
make typecheck       # TypeScript type checking
tsc                  # Compile TypeScript to JavaScript
jco componentize     # Compile JavaScript to Wasm component
make build          # Compose with transport
```

Or simply: `make build` (runs all steps)

### Adding New Tools

Use the `createTool` helper to add tools with type safety:

```typescript
const myTool = createTool({
    name: 'my_tool',
    description: 'Tool description',
    schema: {
        type: 'object',
        properties: {
            param: { type: 'string', description: 'Parameter' }
        },
        required: ['param']
    },
    execute: async (args) => {
        // Tool implementation
        return `Result: ${args.param}`;
    }
});
```

Then add it to the tools array in `createHandler()`. The helper provides:
- Type-safe argument parsing
- Automatic error handling
- JSON schema validation

## Concurrency in JavaScript/Wasm

JavaScript in Wasm Components uses jco's async support, which maps JavaScript Promises to WASI async I/O:

```typescript
// Concurrent fetching works as expected
const results = await Promise.all([
    fetch(url1),
    fetch(url2),
    fetch(url3)
]);
```

Unlike native Node.js, this runs in a Wasm sandbox with:
1. No access to Node.js APIs (fs, process, etc.)
2. Network access controlled by the runtime
3. WASI-based fetch implementation

See the `multiWeather` tool for concurrent HTTP patterns.

## Testing

The Makefile includes comprehensive test targets:

```bash
make test-all        # Run all tests
make test-echo       # Test echo tool
make test-weather    # Test weather tool  
make test-multi      # Test concurrent weather fetching
```

Tests use `curl` to send JSON-RPC requests to the running server. Example:

```bash
# Manual test
curl -X POST http://localhost:8080/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello"}},"id":1}'
```

## Debugging

### Common Issues

**TypeScript compilation errors**
- Run `make typecheck` to see detailed errors
- Ensure all dependencies are installed: `npm install`

**fetch is not defined**
- Use the WASI fetch, not Node.js modules
- Ensure allowed hosts are configured in spin.toml

**Server doesn't start**
- Verify port 8080 is available: `lsof -i :8080`
- Check wasmtime is installed: `which wasmtime`

### Inspecting the Component

```bash
make inspect  # Show component structure and exports
```

## Runtime Options

The server can run on any WASI-compliant runtime:

```bash
# Wasmtime (default)
wasmtime serve -Scli ./mcp-http-server.wasm

# Spin
spin up
```