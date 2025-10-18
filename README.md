<div align="center">

# `wasmcp`

A [WebAssembly Component](https://component-model.bytecodealliance.org/) Development Kit for the [Model Context Protocol](https://modelcontextprotocol.io/docs/getting-started/intro)

</div>

## Quick Start

Author MCP tools in your favorite language
```bash
wasmcp new time-tools --language python

# Develop and build the component
cd time-tools
make # produces time-tools.wasm

# Compose your tools with a Streamable HTTP transport component (default) to form an MCP server
wasmcp compose time-tools.wasm -t http -o http-server.wasm 

# Run
wasmtime serve -Scli http-server.wasm # serves http://0.0.0.0:8080/ by default

# Or compose the same tool components with a stdio transport
wasmcp compose time-tools.wasm -t stdio -o stdio-server.wasm
wasmtime run stdio-server.wasm
```

You can add any number of components together in sequence. If you include multiple tool components, the server will expose the combined set of tools automatically.
```bash
wasmcp new math-tools --language rust

cd math-tools
make

# Use the time-tools.wasm you built earlier
wasmcp compose math-tools.wasm ../time-tools.wasm -o server.wasm
```

## Installation

**Download latest release:**

See [releases](https://github.com/wasmcp/wasmcp/releases) for pre-built binaries.

**Build from source:**
```bash
cargo install --git https://github.com/wasmcp/wasmcp
```

**Prerequisites:**
- [Wasmtime](https://wasmtime.dev/) - WebAssembly runtime (required to run servers)

See [cli/README.md](cli/README.md) for detailed usage.

## Why?

WebAssembly components are:
- **Composable** - Combine compiled binaries like building blocks
- **Sandboxed** - Isolated execution with explicit interfaces
- **Distributable** - Push/pull components from OCI registries
- **Lean** - Complete servers can be under 1MB

These qualities are a perfect match for MCP's [server design principals](https://modelcontextprotocol.io/specification/2025-06-18/architecture#design-principles).

> 1. Servers should be extremely easy to build
> 2. Servers should be highly composable
> 3. Servers should not be able to read the whole conversation, nor “see into” other servers
> 4. Features can be added to servers and clients progressively

## Architecture

Server features like tools, resources, prompts, and completions, are implemented by individual WebAssembly components that export the narrow, spec-mapped WIT interfaces in [wit/protocol/mcp.wit](wit/protocol/mcp.wit).

`wasmcp compose` wraps these components with published middleware components from [crates/](crates/) and composes them together behind a transport component as a complete middleware [chain of responsibility](https://en.wikipedia.org/wiki/Chain-of-responsibility_pattern) that implements an MCP server. The chain terminates with [crates/method-not-found](crates/method-not-found), which returns errors for unhandled methods.

Any of the published default wasmcp components can be swapped out for custom implementations during composition, enabling flexible server configurations.

```
Transport<Protocol>
        ↓
    Middleware₀
        ↓
    Middleware<Feature>₁
        ↓
    Middleware<Feature>₂
        ↓
       ...
        ↓
    Middlewareₙ
        ↓
    MethodNotFound
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

When a client requests `tools/list`, each component that offers tools contributes their tools, creating a unified catalog automatically.

## Components

### Your Components

Write handlers in any language with [component toolchain support](https://component-model.bytecodealliance.org/language-support.html):

```bash
wasmcp new my-handler --language rust       # Rust (calculator example)
wasmcp new my-handler --language python     # Python (string tools example)
wasmcp new my-handler --language typescript # TypeScript (example tool)
```

Generated templates demonstrate the capability pattern with working tool implementations.

### Framework Components

Published to [ghcr.io/wasmcp](https://github.com/orgs/wasmcp/packages):

- **http-transport** - HTTP server (for `wasmtime serve`)
- **stdio-transport** - Stdio integration (for local clients)
- **method-not-found** - Terminal handler for unhandled methods

The CLI automatically downloads these when composing.

**Prerequisites:**
- [Wasmtime](https://wasmtime.dev/) - WebAssembly runtime (required to run servers)

## Examples

See [examples/](examples/) directory for:
- Multi-language compositions
- Handler patterns
- Integration examples

## License

Apache 2.0
