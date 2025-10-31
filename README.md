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

## Runtime Compatibility

wasmcp supports multiple WebAssembly runtimes. Composed servers are automatically configured for your target runtime:

- **Spin** (default) - Uses WASI draft2 components (`@0.2.0-draft2`)
- **wasmtime** - Uses WASI draft components (`@0.2.3`)
- **wasmcloud** - Uses WASI draft components (`@0.2.3`)

The CLI automatically selects the correct framework component variants during composition based on your `--spin`, `--wasmtime`, or `--wasmcloud` flag.

## Quick Start

Create and run your first MCP tool component:
```bash
# Create a component in your favorite language
wasmcp new time-tools --language python
cd time-tools && make && cd ..

# Register it with a short alias
wasmcp registry component add time time-tools/time-tools.wasm

# Compose into an MCP server
wasmcp compose server time -o server.wasm

# Run with your target runtime:
spin up server.wasm              # Spin (default)
wasmtime serve -Scli server.wasm # wasmtime
wash up server.wasm              # wasmcloud
```

**Target specific runtimes during composition:**
```bash
# Spin (default - uses draft2 variants)
wasmcp compose server time -o server.wasm

# wasmtime (uses draft variants)
wasmcp compose server time --wasmtime -o server.wasm

# wasmcloud (uses draft variants)
wasmcp compose server time --wasmcloud -o server.wasm
```

Combine multiple tool components - they automatically merge into a unified catalog:
```bash
# Create another component
wasmcp new math-tools --language rust
cd math-tools && make && cd ..
wasmcp registry component add math math-tools/target/wasm32-wasip2/release/math_tools.wasm

# Compose both together (Spin is default)
wasmcp compose server time math -o combined-server.wasm
spin up combined-server.wasm
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

Server features like tools, resources, prompts, and completions, are implemented by individual WebAssembly components that export the narrow, spec-mapped WIT interfaces defined in [spec/2025-06-18/wit/](spec/2025-06-18/wit/).

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
wasmcp compose server ./calculator.wasm -o server.wasm

# Registry package (OCI) - colon identifies it as a registry spec
wasmcp compose server wasmcp:calculator@0.1.0 -o server.wasm

# Aliases (registered in ~/.config/wasmcp/wasmcp.toml)
wasmcp compose server calc weather -o server.wasm

# Mixed: local path + registry package + alias
wasmcp compose server ./logger.wasm wasmcp:calculator@1.0 weather -o server.wasm
```

When a client requests `tools/list`, each component that offers tools contributes their tools, creating a unified catalog automatically.

### Runtime Targeting

wasmcp automatically selects the correct component variants based on your target runtime.

#### CLI Flags

Specify your target runtime with mutually exclusive flags:

```bash
# Spin runtime (WASI draft2 @0.2.0-draft2) - DEFAULT
wasmcp compose server components... -o server.wasm
wasmcp compose server components... --spin -o server.wasm

# wasmtime runtime (WASI draft @0.2.3)
wasmcp compose server components... --wasmtime -o server.wasm

# wasmcloud runtime (WASI draft @0.2.3)
wasmcp compose server components... --wasmcloud -o server.wasm
```

#### Component Variants

Framework components are published in two variants:

| Component | Draft Variant | Draft2 Variant | Auto-Selected By |
|-----------|---------------|----------------|------------------|
| http-transport | `wasmcp:http-transport@X.X.X` | `wasmcp:http-transport-d2@X.X.X` | `--wasmtime`, `--wasmcloud` \| `--spin` (default) |
| sessions | `wasmcp:sessions@X.X.X` | `wasmcp:sessions-d2@X.X.X` | `--wasmtime`, `--wasmcloud` \| `--spin` (default) |

The CLI automatically downloads the correct variant based on your runtime flag. You never need to specify the `-d2` suffix manually.

#### Session Support

If your components import the sessions interface, wasmcp automatically:
1. Detects session usage by inspecting component imports
2. Includes the matching sessions variant in composition
3. Validates consistent WASI draft usage across all components

**Example:**
```bash
# Component imports wasmcp:mcp-v20250618/sessions@0.1.3
# CLI detects session usage and includes sessions-d2 (because Spin is default)
wasmcp compose server my-stateful-component.wasm -o server.wasm

# Override to wasmtime - CLI includes sessions (draft variant)
wasmcp compose server my-stateful-component.wasm --wasmtime -o server.wasm
```

#### Mixed Draft Error

Components using different WASI draft versions cannot be composed together:

```bash
# ERROR: Component A uses draft2, Component B uses draft
wasmcp compose server componentA.wasm componentB.wasm

# Error output:
# Mixed WASI draft versions detected:
# Previous component used Draft2, but componentB uses Draft
# All components must use the same WASI draft version.
```

Rebuild incompatible components targeting the same WASI version.

## Migration Guide

### Upgrading from v0.4.x (Pre-Sessions Branch)

**Breaking Change:** The default runtime target has changed from **wasmtime → Spin**.

#### If You Previously Used wasmtime (Default Before v0.5.0)

**Old behavior (v0.4.x):**
```bash
wasmcp compose server components... -o server.wasm
# Used draft variants automatically (wasmtime assumed)
wasmtime serve -Scli server.wasm
```

**New behavior (v0.5.0+):**
```bash
# Now defaults to Spin (draft2 variants)
wasmcp compose server components... -o server.wasm
spin up server.wasm

# To preserve wasmtime behavior, use --wasmtime flag
wasmcp compose server components... --wasmtime -o server.wasm
wasmtime serve -Scli server.wasm
```

**Action Required:** Add `--wasmtime` flag to existing workflows if you target wasmtime runtime.

#### If You Deploy to Spin

**Old behavior (v0.4.x):**
```bash
wasmcp compose server components... -o server.wasm
# May have failed if components used draft WASI versions
# Had to manually rebuild components for draft2
```

**New behavior (v0.5.0+):**
```bash
# Automatically uses draft2 variants for Spin
wasmcp compose server components... -o server.wasm  # Or --spin
spin up server.wasm  # Just works!
```

**Action Required:** None - Spin is now the default and will "just work".

#### If You Use OCI Registry Packages

**Component naming has changed:**

**Old (v0.4.x):**
- Single variant: `wasmcp:http-transport@0.1.3`
- Ambiguous which WASI draft version

**New (v0.5.0+):**
- Draft variant: `wasmcp:http-transport@0.1.4` (for wasmtime/wasmcloud)
- Draft2 variant: `wasmcp:http-transport-d2@0.1.4` (for Spin)
- CLI automatically selects based on runtime flag

**Action Required:** Update package specs to include version, let CLI handle variant selection:
```bash
# Old (ambiguous)
wasmcp compose server wasmcp:http-transport

# New (explicit version, automatic variant)
wasmcp compose server --wasmtime ...  # Auto-downloads http-transport@0.1.4
wasmcp compose server --spin ...      # Auto-downloads http-transport-d2@0.1.4
```

You never manually specify `-d2` suffix - the CLI handles it based on runtime flags.

#### Session Support (New Feature)

**What's New:**
- Automatic detection of session imports
- Transparent inclusion of correct sessions variant
- No manual configuration required

**If your components import sessions:**
```wit
import wasmcp:mcp-v20250618/sessions@0.1.3;
```

The CLI automatically:
1. Detects the sessions import
2. Includes `sessions` (draft) or `sessions-d2` (draft2) based on runtime flag
3. Validates consistent WASI draft usage

**No action required** - sessions work automatically.

#### Summary of Changes

| Change | v0.4.x Behavior | v0.5.0+ Behavior | Migration Action |
|--------|-----------------|------------------|------------------|
| Default runtime | wasmtime (implicit) | Spin (explicit) | Add `--wasmtime` if needed |
| Component variants | Single variant | Draft + Draft2 | Let CLI auto-select |
| Session support | Manual | Automatic | None - automatic detection |
| Package naming | `component@version` | `component@version` + `component-d2@version` | Use runtime flags, not suffixes |

#### Testing Your Migration

```bash
# Test Spin (new default)
wasmcp compose server your-component.wasm -o server.wasm -v
spin up server.wasm

# Test wasmtime (preserve old behavior)
wasmcp compose server your-component.wasm --wasmtime -o server.wasm -v
wasmtime serve -Scli server.wasm

# Verify correct variant in verbose output
# Spin should show: "Downloading wasmcp:http-transport-d2@..."
# wasmtime should show: "Downloading wasmcp:http-transport@..."
```

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
wasmcp compose server calc weather -o server.wasm
wasmcp compose server db logger -o server.wasm

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
wasmcp compose server dev
# Creates: ~/.config/wasmcp/composed/dev.wasm

# Or specify a different output location
wasmcp compose server dev -o ./my-server.wasm
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

Published to [ghcr.io/wasmcp](https://github.com/orgs/wasmcp/packages) in two variants:

**Draft variants (wasmtime, wasmcloud - WASI @0.2.3):**
- **http-transport** - HTTP server transport
- **stdio-transport** - Stdio transport
- **sessions** - Session management (WASI KV backed)
- **tools-middleware** - Tools capability wrapper
- **resources-middleware** - Resources capability wrapper
- **prompts-middleware** - Prompts capability wrapper
- **method-not-found** - Terminal handler

**Draft2 variants (Spin - WASI @0.2.0-draft2):**
- **http-transport-d2** - HTTP server transport
- **sessions-d2** - Session management (WASI KV backed)

**Runtime-agnostic components (single variant):**
- **stdio-transport**, **tools-middleware**, **resources-middleware**, **prompts-middleware**, **method-not-found**

The CLI automatically downloads the correct variant based on `--spin`, `--wasmtime`, or `--wasmcloud` flags.

## License

Apache 2.0
