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
cargo install --git https://github.com/wasmcp/wasmcp wasmcp
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
wasmcp compose server time --runtime wasmtime -o server.wasm
wasmtime serve -Scli -Skeyvalue -Shttp server.wasm  # http://0.0.0.0:8080/mcp
```

Combine multiple tool components - they automatically merge into a unified catalog:
```bash
# Create another component
wasmcp new math-tools --language rust
cd math-tools && make && cd ..
wasmcp registry component add math math-tools/target/wasm32-wasip2/release/math_tools.wasm

# Compose both together
wasmcp compose server time math --runtime wasmtime -o combined-server.wasm
wasmtime serve -Scli -Skeyvalue -Shttp combined-server.wasm
```

See [examples/](examples/) for more.

## Documentation

- **[Examples](examples/)**
- **[CLI Reference](cli/README.md)**
- **[Development MCP server](docs/daemon-management.md)** - Run a local development server that provides context to your coding agent about developing, composing, and running `wasmcp` projects.

## Authentication Modes

wasmcp supports both public (unauthenticated) and OAuth 2.1 protected MCP servers via the `WASMCP_AUTH_MODE` environment variable.

### Public Mode (Default)

```bash
# No environment variables needed (default behavior)
wasmtime serve -Scli -Skeyvalue -Shttp server.wasm
```

Or explicitly set:
```bash
WASMCP_AUTH_MODE=public wasmtime serve -Scli -Skeyvalue -Shttp server.wasm
```

### OAuth Mode

Requires JWT bearer tokens per MCP OAuth 2.1 spec. Supports two validation patterns:

#### Dynamic Registration Pattern

Per-user client IDs created dynamically. No fixed audience - validation via issuer and signature only.

**Required Environment Variables:**
- `WASMCP_AUTH_MODE=oauth` - Enable OAuth authentication
- `JWT_ISSUER` - Expected token issuer (e.g., `https://your.issuer.com`)
- `JWT_JWKS_URI` - JWKS endpoint for public key retrieval

**Example:**
```bash
WASMCP_AUTH_MODE=oauth \
JWT_ISSUER=https://api.workos.com \
JWT_JWKS_URI=https://api.workos.com/sso/jwks/client_01234567890 \
wasmtime serve -Scli -Skeyvalue -Shttp server.wasm
```

## Environment Variables

All wasmcp servers support runtime configuration via environment variables. These control transport behavior, authentication, sessions, and security policies.

> **See also**: [CLI Reference](cli/README.md) for detailed runtime configuration and differences between Spin and Wasmtime.

### HTTP Transport Mode

Control whether HTTP transport uses Server-Sent Events (SSE) for streaming or plain JSON mode.

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `WASMCP_DISABLE_SSE` | Disable SSE streaming, use plain JSON responses | `false` | No |

**When to use**: Set to `true` for clients that don't support Server-Sent Events. In plain JSON mode, notifications are suppressed and only a single response is sent per request.

**Example**:
```bash
# Plain JSON mode (no streaming)
WASMCP_DISABLE_SSE=true wasmtime serve -Scli -Skeyvalue -Shttp server.wasm
```

### Session Management

> ⚠️ **Warning**: Sessions require the **Spin runtime**. Wasmtime spawns new component instances per request and doesn't persist the key-value store across requests. Sessions created in one request won't be available in subsequent requests when using Wasmtime.

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `WASMCP_SESSION_ENABLED` | Enable HTTP session tracking via `Mcp-Session-Id` header | `false` | No |
| `WASMCP_SESSION_BUCKET` | Key-value bucket name for session storage | `default` | No |

**Example** (Spin runtime required):
```bash
# Compose for Spin runtime
wasmcp compose server calculator.wasm --runtime spin -o server.wasm

# Configure in spin.toml:
# [component.mcp.environment]
# WASMCP_SESSION_ENABLED = "true"
# WASMCP_SESSION_BUCKET = "default"
#
# [component.mcp]
# key_value_stores = ["default"]

spin up
```

### Authentication & Authorization

Configure OAuth 2.1 / JWT bearer token validation per the [MCP OAuth spec](https://spec.modelcontextprotocol.io/2025-06-18/architecture#oauth).

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `WASMCP_AUTH_MODE` | Authentication mode: `public` or `oauth` | `public` | No |
| `JWT_ISSUER` | Expected JWT issuer URL | - | Yes (when `oauth`) |
| `JWT_JWKS_URI` | JWKS endpoint for JWT public key retrieval | - | Yes (when `oauth`)* |
| `JWT_PUBLIC_KEY` | PEM-encoded public key (alternative to JWKS) | - | No |
| `JWT_AUDIENCE` | Expected JWT audience claim (server URI) | - | No** |
| `JWT_REQUIRED_SCOPES` | Comma-separated required OAuth scopes | - | No |
| `JWT_JWKS_TTL` | JWKS cache TTL in seconds | `300` | No |

\* Either `JWT_JWKS_URI` or `JWT_PUBLIC_KEY` required when `WASMCP_AUTH_MODE=oauth`

\*\* Only required for traditional OAuth pattern. Do NOT set for dynamic registration flows (e.g., WorkOS) where audience is the per-user client ID.

**Example** (OAuth with dynamic registration):
```bash
WASMCP_AUTH_MODE=oauth \
JWT_ISSUER=https://divine-lion-50.authkit.app \
JWT_JWKS_URI=https://divine-lion-50.authkit.app/oauth2/jwks \
JWT_JWKS_TTL=300 \
wasmtime serve -Scli -Skeyvalue -Shttp server.wasm
```

### CORS & Security

Control Origin header validation to prevent DNS rebinding attacks.

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `WASMCP_ALLOWED_ORIGINS` | Comma-separated allowed Origin values, or `*` for all | localhost only | No |
| `WASMCP_REQUIRE_ORIGIN` | Require Origin header on all requests | `false` | No |

**Default allowed origins**: `http://127.0.0.1`, `http://localhost`, `http://[::1]`

**Example**:
```bash
# Allow specific origins
WASMCP_ALLOWED_ORIGINS=https://app.example.com,https://admin.example.com \
wasmtime serve -Scli -Skeyvalue -Shttp server.wasm

# Allow all origins (use with caution)
WASMCP_ALLOWED_ORIGINS=* wasmtime serve -Scli -Skeyvalue -Shttp server.wasm
```

> **Note**: Most MCP desktop clients don't send Origin headers. Only enable `WASMCP_REQUIRE_ORIGIN` if all your clients are browser-based.

### Discovery & Metadata

Configure OAuth Protected Resource metadata per [RFC 9728](https://datatracker.ietf.org/doc/html/rfc9728).

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `WASMCP_SERVER_URI` | Server's canonical URI (resource identifier) | Host header | No |
| `WASMCP_AUTH_SERVER_URL` | Authorization server URL for discovery metadata | `JWT_ISSUER` | No |
| `WASMCP_DISCOVERY_CACHE_TTL` | Cache TTL for `/.well-known/*` endpoints (seconds) | `3600` | No |

These variables control the OAuth discovery endpoint (`/.well-known/oauth-protected-resource`) and `WWW-Authenticate` challenge headers.

**Example**:
```bash
WASMCP_SERVER_URI=https://mcp.example.com \
WASMCP_AUTH_SERVER_URL=https://auth.example.com \
WASMCP_DISCOVERY_CACHE_TTL=300 \
wasmtime serve -Scli -Skeyvalue -Shttp server.wasm
```

## Features

- **Stateful Sessions** - Built-in session management with key-value storage for multi-request workflows
- **Authentication** - JWT/OAuth bearer token validation with scope-based authorization
- **Auto-Composition** - Automatically wraps components with appropriate middleware
- **Type-Safe Storage** - TypedValue enum for runtime type safety in sessions
- **Real-time Notifications** - Progress updates, logs, and resource changes via streaming

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

Published to [ghcr.io/wasmcp](https://github.com/orgs/wasmcp/packages):

- **transport** - Universal transport for HTTP / stdio execution with JWT validation
- **server-io** - Universal MCP message I/O with configurable transport framing support
- **session-store** - Stateful session management with key-value storage
- **authorization** - JWT/OAuth bearer token validation and claim extraction
- **kv-store** - Type-safe key-value storage with TypedValue support
- **method-not-found** - Terminal handler for unhandled methods

The CLI automatically downloads these when composing.

## License

Apache 2.0
