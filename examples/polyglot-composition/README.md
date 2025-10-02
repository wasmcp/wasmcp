# Polyglot Composition Example

Demonstrates composing MCP server components written in four different languages into a single WebAssembly binary.

## Components

- **TypeScript Middleware** (`ts-logger`) - Request logging middleware
- **Go Middleware** (`go-logger`) - Authentication/monitoring middleware
- **Rust Tools Handler** (`rust-tools`) - Tools capability handler
- **Python Resources Handler** (`python-resources`) - Resources capability handler

## Quick Start

```bash
# Build and compose
make compose

# Run server
make run

# Test endpoints
make test
```

## Building

Build all components:

```bash
make build
```

Or build individually:

```bash
cd rust-tools && make build
cd python-resources && make build
cd ts-logger && make build
cd go-logger && make build
```

## Composition

Compose all components with explicit paths:

```bash
make compose
```

This creates `mcp-server.wasm` (~47MB with Python runtime).

The Makefile uses explicit composition:
```bash
wasmcp compose \
  --middleware ts-logger/target/ts_logger.wasm \
  --middleware go-logger/target/go_logger.wasm \
  --tools rust-tools/target/wasm32-wasip1/release/rust_tools.wasm \
  --resources python-resources/target/python_resources.wasm \
  -o mcp-server.wasm
```

## Execution Chain

```
HTTP Request
    ↓
http-transport
    ↓
ts-logger (logs to stderr)
    ↓
go-logger (logs to stdout)
    ↓
rust-tools (handles tools/*, returns)
    ↓
python-resources (handles resources/*, returns)
    ↓
initialize-handler (handles initialize, returns)
```

**Key Point:** Middleware positioned after handlers won't see requests those handlers process. Both middleware are placed first to observe all requests.

## Running

```bash
wasmtime serve -Scommon mcp-server.wasm
```

Server listens on `http://0.0.0.0:8080`

## Testing

### Initialize
```bash
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
```

### List Tools
```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
```

### Call Tool
```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
      "name": "echo",
      "arguments": {"message": "Hello!"}
    }
  }'
```

### List Resources
```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":4,"method":"resources/list","params":{}}'
```

### Read Resource
```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 5,
    "method": "resources/read",
    "params": {"uri": "file:///example.txt"}
  }'
```

## Architecture

### Language Characteristics

**Rust** (~220KB)
- Smallest, fastest
- Production hot paths

**Go** (~340KB)
- Small TinyGo runtime
- Fast compilation

**TypeScript** (~11MB)
- QuickJS runtime
- npm ecosystem

**Python** (~36MB)
- componentize-py runtime
- Rapid prototyping

### Streaming & Memory

All components write directly to output stream:
- Constant memory usage
- Backpressure via `check_write()`
- Edge-ready architecture

### Development to Production

1. Prototype in Python
2. Profile hot paths
3. Rewrite in Rust
4. Deploy incrementally

WIT interfaces guarantee compatibility.

## Deployment Patterns

### Edge Tiers

**Development** (47MB):
```bash
# All components, rapid iteration
make compose
```

**Production** (1.5MB):
```bash
# Rust only for performance
wasmcp compose --tools rust-tools.wasm --resources rust-resources.wasm
```

### Regional

```bash
# US: High tools traffic
wasmcp compose --tools rust-tools.wasm

# EU: High resources traffic
wasmcp compose --resources python-resources.wasm
```

## Version Compatibility

All components use `wasmcp@0.3.0-alpha.59`:

```bash
wasmcp new --version 0.3.0-alpha.59 <handler>
wasmcp compose --version 0.3.0-alpha.59 -o server.wasm
```

## License

Apache 2.0
