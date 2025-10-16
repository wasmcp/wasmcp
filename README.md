<div align="center">

# `wasmcp`

Build [Model Context Protocol](https://modelcontextprotocol.io/) servers using composable [WebAssembly Components](https://component-model.bytecodealliance.org/)

</div>

## Quick Start

```bash
# Create a new handler
wasmcp new my-handler --language python

# Build the component
cd my-handler
make setup && make build

# Compose into a server
wasmcp compose target/my-handler.wasm -o server.wasm

# Run
wasmtime serve -Scli server.wasm
```

See [cli/README.md](cli/README.md) for detailed usage.

## Why?

WebAssembly components are:
- **Composable** - Combine compiled binaries like building blocks
- **Sandboxed** - Isolated execution with explicit interfaces
- **Distributable** - Push/pull from OCI registries
- **Lean** - Complete servers under 1MB

These qualities align perfectly with MCP's modular server architecture.

## Architecture

All components implement the universal `wasmcp:mcp/server-handler` interface, forming a simple linear pipeline:

```
transport → component₁ → component₂ → ... → method-not-found
```

Each component:
- Handles requests it understands (e.g., `tools/call`)
- Delegates others downstream
- Merges results (e.g., combining tool lists)

This enables dynamic composition without complex configuration - like Unix pipes for MCP.

### Example Composition

```bash
# Single calculator handler
wasmcp compose calculator.wasm -o server.wasm

# Multiple handlers in a pipeline
wasmcp compose logger.wasm calculator.wasm weather.wasm -o server.wasm
```

When a client requests `tools/list`, each component contributes its tools, creating a unified catalog automatically.

## Components

### Your Components

Write handlers in any language with [component toolchain support](https://component-model.bytecodealliance.org/language-support.html):

```bash
wasmcp new my-handler --language rust    # Rust
wasmcp new my-handler --language python  # Python
```

Generated templates implement a simple calculator demonstrating the handler pattern.

### Framework Components

Published to [ghcr.io/wasmcp](https://github.com/orgs/wasmcp/packages):

- **http-transport** - HTTP server (for `wasmtime serve`)
- **stdio-transport** - Stdio integration (for local clients)
- **method-not-found** - Terminal handler for unhandled methods

The CLI automatically downloads these when composing.

## Installation

**Download latest release:**

See [releases](https://github.com/wasmcp/wasmcp/releases) for pre-built binaries.

**Build from source:**
```bash
cd cli
cargo build --release --target <your-host-triple>
```

**Prerequisites:**
- [Wasmtime](https://wasmtime.dev/) - WebAssembly runtime (required to run servers)

## Examples

See [examples/](examples/) directory for:
- Multi-language compositions
- Handler patterns
- Integration examples

## License

Apache 2.0
