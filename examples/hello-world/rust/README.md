# Hello World - Rust MCP Server

A minimal Model Context Protocol (MCP) server with a single echo tool, written in Rust.

## What This Does

Provides one tool:
- **echo**: Takes a message and echoes it back

## Prerequisites

- Rust 1.75+
- `cargo-component` (`cargo install cargo-component`)
- `wasmcp` CLI (`cargo install wasmcp`)
- `wasmtime` runtime

## Quick Start

```bash
# 1. Fetch WIT dependencies
wkg wit fetch

# 2. Build the component
cargo component build --release

# 3. Compose the MCP server
wasmcp compose --tools target/wasm32-wasip1/release/rust.wasm -o mcp-server.wasm

# 4. Run it
wasmtime serve -Scommon mcp-server.wasm
```

Server runs on `http://0.0.0.0:8080`

Or use the Makefile: `make run`

## Testing

Using the official MCP Inspector CLI:

```bash
# List tools
npx @modelcontextprotocol/inspector@0.16.8 --cli http://localhost:8080/mcp \
  --method tools/list

# Call echo tool
npx @modelcontextprotocol/inspector@0.16.8 --cli http://localhost:8080/mcp \
  --method tools/call \
  --tool-name echo \
  --tool-arg message="Hello World!"
```

**Note:** The server uses the standard MCP streamable-http transport endpoint at `POST /mcp`.

Or use the Makefile: `make server-test` (requires server running in another terminal)

## How It Works

1. **src/lib.rs** - Rust code implementing the echo tool
2. **wit/world.wit** - Interface definition (WIT)
3. **cargo-component** - Compiles Rust → WebAssembly component
4. **wasmcp compose** - Wires component with MCP infrastructure
5. **wasmtime serve** - Runs the composed server

## File Structure

```
rust/
├── src/
│   └── lib.rs       # Tool implementation
├── wit/
│   └── world.wit    # WIT interface
├── Cargo.toml       # Rust package manifest
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
