# Hello World - Python MCP Server

A minimal Model Context Protocol (MCP) server with a single echo tool, written in Python.

## What This Does

Provides one tool:
- **echo**: Takes a message and echoes it back

## Prerequisites

- Python 3.10+
- `wasmcp` CLI (`cargo install wasmcp`)
- `wasmtime` runtime

## Quick Start

```bash
# 1. Setup Python environment
python3 -m venv venv
venv/bin/pip install componentize-py

# 2. Fetch WIT dependencies
wkg wit fetch

# 3. Generate Python bindings
venv/bin/componentize-py --wit-path wit --world tools-handler bindings .

# 4. Build the component
venv/bin/componentize-py \
  --wit-path wit \
  --world tools-handler \
  componentize app \
  -o target/hello_world.wasm

# 5. Compose the MCP server
wasmcp compose --tools target/hello_world.wasm -o mcp-server.wasm

# 6. Run it
wasmtime serve -Scommon mcp-server.wasm
```

Server runs on `http://0.0.0.0:8080`

Or use the Makefile: `make run`

## Testing

```bash
# Initialize
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
      "protocolVersion": "2024-11-05",
      "capabilities": {},
      "clientInfo": {"name": "test", "version": "1.0"}
    }
  }'

# List tools
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'

# Call echo tool
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
      "name": "echo",
      "arguments": "{\"message\":\"Hello World!\"}"
    }
  }'
```

## How It Works

1. **app.py** - Python code implementing the echo tool
2. **wit/world.wit** - Interface definition (WIT)
3. **componentize-py** - Compiles Python → WebAssembly component
4. **wasmcp compose** - Wires component with MCP infrastructure
5. **wasmtime serve** - Runs the composed server

## File Structure

```
hello-world/
├── app.py           # Tool implementation
├── wit/
│   └── world.wit    # WIT interface
├── Makefile         # Build automation
└── README.md        # This file
```

## Makefile

A Makefile is provided for convenience:

```bash
make run    # Build, compose, and run
make test   # Test the server
make clean  # Clean all artifacts
```
