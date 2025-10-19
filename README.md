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

### Simplify with Component Aliases

Register frequently-used components with short aliases:
```bash
# Register aliases for easier composition
wasmcp registry component add time ../time-tools/time-tools.wasm
wasmcp registry component add math math-tools.wasm

# Now compose using short names
wasmcp compose time math -o server.wasm
```

### Reusable Profiles

Create profiles for common server configurations:
```bash
# Create a development profile
wasmcp registry profile add dev time math -o dev-server.wasm

# Compose your entire dev stack with one command
wasmcp compose dev

# Extend profiles with additional components
wasmcp compose dev logger-tools.wasm -o debug-server.wasm
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

## Registry

The registry system provides component aliases and profiles for simplified workflows.

### Component Aliases

Register short names for frequently-used components:

```bash
# Register local components
wasmcp registry component add calc ./calculator.wasm
wasmcp registry component add weather ./weather-tools.wasm

# Register from registry
wasmcp registry component add db wasmcp:database@1.0.0

# Use in composition
wasmcp compose calc weather -o server.wasm

# List and manage
wasmcp registry component list
wasmcp registry component remove calc
```

**Aliases support:**
- Local file paths (automatically canonicalized to absolute paths)
- Registry package specs (e.g., `wasmcp:calculator@0.1.0`)
- Alias chaining (aliases can reference other aliases)

### Profiles

Create reusable composition pipelines:

```bash
# Create a base profile
wasmcp registry profile add base calc weather -o base.wasm

# Create profile with inheritance
wasmcp registry profile add prod logger monitor -o prod.wasm -b base

# Use profiles in composition
wasmcp compose base                    # Uses base profile's output path
wasmcp compose prod extra-tool.wasm    # Extends prod profile

# List and manage
wasmcp registry profile list
wasmcp registry profile remove base
```

**Profile features:**
- Inheritance chains (profiles can extend base profiles)
- Automatic output path resolution
- Mix with direct components: `wasmcp compose my-profile extra-comp`

### Unified Resolution

Profiles and components work seamlessly together - just list what you want:

```bash
# Mix profiles and components freely
wasmcp compose base-profile custom-tool weather-profile -o server.wasm

# Order is preserved: base components → custom-tool → weather components
```

**Resolution order:**
1. If spec matches a profile name → expand profile components in-place
2. Otherwise resolve as component (alias → path → registry package)

**Validation:**
- Component aliases and profile names must be unique
- Enforced at registration time with clear error messages
- Circular dependencies detected in both aliases and profile inheritance

### Registry Info

View your registry configuration:

```bash
wasmcp registry info              # Show all
wasmcp registry info --components # Filter to components
wasmcp registry info --profiles   # Filter to profiles
```

### Configuration

Registry data is stored in `~/.config/wasmcp/config.toml` (XDG Base Directory compliant).

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
