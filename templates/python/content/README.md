# {{project-name}}

{{project-description}}

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
- **Provider**: Your Python implementation of MCP tools (this code)
- **Transport**: Pre-built HTTP server component from the registry

The composition happens at build time, producing a single `mcp-http-server.wasm` that can run on any runtime that supports the Wasm component model.

## Development

### Prerequisites

- **Python 3.10+** - Required for componentize-py
- **componentize-py** - Compiles Python to Wasm components
- **wasm-tools** - Component model toolchain

Quick setup:
```bash
make setup  # Checks and installs all dependencies
```

### Project Structure

```
├── app.py           # Tool implementations with decorators
├── helpers.py       # MCP SDK-like helper decorators
├── wit/             # WebAssembly Interface Types
├── wit_world/       # Generated bindings (don't edit)
└── Makefile         # Build automation
```

### Build Pipeline

The build process has three stages:

```bash
make bindgen         # Generate Python bindings from WIT
make build-provider  # Compile Python to Wasm component
make build          # Compose with transport
```

Or simply: `make build` (runs all steps)

### Adding New Tools

Use the decorator-based API to add tools:

```python
@tool(
    name="my_tool",
    description="Tool description",
    input_schema={
        "type": "object",
        "properties": {
            "param": {"type": "string", "description": "Parameter"}
        },
        "required": ["param"]
    }
)
def handle_my_tool(args: dict) -> str:
    """Tool implementation"""
    return f"Result: {args['param']}"
```

The decorator automatically:
- Registers the tool with the server
- Handles JSON schema validation
- Manages error handling

## Async HTTP in Python/Wasm

Python in Wasm uses componentize-py's `poll_loop` for async operations. Unlike native Python's `asyncio`, this integrates with WASI's polling mechanism:

```python
from componentize.poll_loop import poll_loop

async def fetch_data(url: str) -> dict:
    # Uses WASI HTTP bindings
    response = await poll_loop.send(request)
    return response
```

The `poll_loop`:
1. Yields control to the WASI runtime during I/O
2. Enables concurrent requests without threads
3. Works around Python's GIL limitations in Wasm

See the weather example for concurrent HTTP fetching patterns.

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

**ImportError in generated bindings**
- Run `make bindgen` to regenerate bindings after WIT changes
- Ensure virtual environment is activated

**HTTP requests fail or timeout**
- componentize-py must be configured with network access
- Check that URLs are accessible from your environment

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

# Wasmer
wasmer run --net --mapdir /::/ ./mcp-http-server.wasm

# Node.js with WASI
node --experimental-wasi-unstable-preview1 ./run.js
```