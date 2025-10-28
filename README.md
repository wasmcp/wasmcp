<div align="center">

# `wasmcp`

A [WebAssembly Component](https://component-model.bytecodealliance.org/) Development Kit for the [Model Context Protocol](https://modelcontextprotocol.io/docs/getting-started/intro)

</div>

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/wasmcp/wasmcp/main/install.sh | bash
```

See [releases](https://github.com/wasmcp/wasmcp/releases) for SBOMs etc.

Or build from source:

```bash
cargo install --git https://github.com/wasmcp/wasmcp
```

Requires [`wasmtime`](https://wasmtime.dev/), [`wash`](https://github.com/wasmCloud/wash), [`spin`](https://github.com/spinframework/spin), or another component-capable runtime to run composed servers.

## Quick Start

Create and run your first MCP tool component:
```bash
# Create a component in your favorite language
wasmcp new time-tools --language python
cd time-tools && make && cd ..

# Register it with a short alias
wasmcp registry component add time time-tools/time-tools.wasm

# Compose into an MCP server and run
wasmcp compose server time -o server.wasm
wasmtime serve -Scli server.wasm  # http://0.0.0.0:8080/mcp
```

Combine multiple tool components - they automatically merge into a unified catalog:
```bash
# Create another component
wasmcp new math-tools --language rust
cd math-tools && make && cd ..
wasmcp registry component add math math-tools/target/wasm32-wasip2/release/math_tools.wasm

# Compose both together
wasmcp compose server time math -o combined-server.wasm
wasmtime serve -Scli combined-server.wasm
```

See [examples/](examples/) for more.

## Documentation

- **[Examples](examples/)**
- **[CLI Reference](cli/README.md)**
- **[Development MCP server](docs/daemon-management.md)** - Run a local development server that provides context to your coding agent about developing, composing, and running `wasmcp` projects.

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

Components can be specified as local paths, registry packages (OCI), aliases, or profiles:

```bash
# Local file path
wasmcp compose ./calculator.wasm -o server.wasm

# Registry package (OCI) - colon identifies it as a registry spec
wasmcp compose wasmcp:calculator@0.1.0 -o server.wasm

# Aliases (registered in ~/.config/wasmcp/wasmcp.toml)
wasmcp compose calc weather -o server.wasm

# Mixed: local path + registry package + alias
wasmcp compose ./logger.wasm wasmcp:calculator@1.0 weather -o server.wasm
```

When a client requests `tools/list`, each component that offers tools contributes their tools, creating a unified catalog automatically.

## Registry

`wasmcp registry` allows for simple artifact aliases and reusable composition profiles.

### Component Aliases

Register short names for frequently-used components:

```bash
# Register local components (file paths)
wasmcp registry component add calc ./calculator.wasm
wasmcp registry component add weather ./weather-tools.wasm

# Register from OCI registry (namespace:name@version)
wasmcp registry component add db wasmcp:database@1.0.0
wasmcp registry component add logger namespace:logger@2.0.0

# Aliases can also reference other aliases
wasmcp registry component add prod-calc calc

# Use aliases in composition
wasmcp compose calc weather -o server.wasm
wasmcp compose db logger -o server.wasm

# List and manage
wasmcp registry component list
wasmcp registry component remove calc
```

### Profiles

Save a list of components to compose together:

```bash
# Save: dev = calc + weather
wasmcp registry profile add dev calc weather -o dev.wasm

# Later, rebuild the same server
wasmcp compose dev
# Creates: ~/.config/wasmcp/composed/dev.wasm

# Or specify a different output location
wasmcp compose dev -o ./my-server.wasm
# Creates: ./my-server.wasm
```

Profiles can inherit from other profiles:
```bash
wasmcp registry profile add prod logger monitor -o prod.wasm -b dev
# prod = calc + weather + logger + monitor
```

List and remove:
```bash
wasmcp registry profile list
wasmcp registry profile remove dev
```

### Registry Info

View your registry configuration:

```bash
wasmcp registry info              # Show all
wasmcp registry info --components # Filter to components
wasmcp registry info --profiles   # Filter to profiles
```

### Configuration

Registry data is stored in `~/.config/wasmcp/config.toml` ([XDG Base Directory](https://specifications.freedesktop.org/basedir-spec/latest/)).

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

## License

Apache 2.0
