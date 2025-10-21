# wasmcp CLI

Build composable MCP servers using WebAssembly components.

## Installation

```bash
cd cli
cargo build --release
```

The CLI is a native binary (not WebAssembly). It builds for your host platform automatically.

Requires [Wasmtime](https://wasmtime.dev/) to run composed servers.

## Usage

### Create a new component

```bash
wasmcp new my-math --language rust
wasmcp new my-strings --language python
```

Generated projects include simple tool implementations demonstrating the capability pattern.

### Component Registry

Register aliases for frequently-used components:

```bash
# Register components with short names
wasmcp registry component add calc ./calculator.wasm
wasmcp registry component add strings wasmcp:string-tools@1.0.0

# List registered components
wasmcp registry component list

# Remove a component alias
wasmcp registry component remove calc
```

### Profiles

Create reusable composition pipelines:

```bash
# Create a profile
wasmcp registry profile add dev calc strings -o dev-server.wasm

# Create profile with inheritance
wasmcp registry profile add prod monitor -o prod.wasm -b dev

# List profiles
wasmcp registry profile list

# Remove a profile
wasmcp registry profile remove dev
```

### Compose components into a server

```bash
# Using file paths
wasmcp compose component.wasm -o server.wasm

# Using aliases
wasmcp compose calc strings -o server.wasm

# Using profiles
wasmcp compose dev

# Mix profiles and components (order preserved)
wasmcp compose dev extra-component.wasm -o server.wasm

# Transport options
wasmcp compose component.wasm -t stdio -o server.wasm
wasmcp compose component.wasm -t http -o server.wasm

# Force overwrite existing output
wasmcp compose calc strings -o server.wasm --force

# Verbose output for debugging
wasmcp compose calc strings -o server.wasm --verbose

# Advanced options
wasmcp compose calc --deps-dir ./my-deps --skip-download
wasmcp compose calc --override-transport custom-transport.wasm
wasmcp compose calc --override-method-not-found custom-handler.wasm
```

The CLI automatically detects component types and wraps them with appropriate middleware.

**Resolution order:** Each spec is checked as profile → alias → path → registry package. Profiles expand in-place, preserving component order.

### Run the server

```bash
# HTTP (default)
wasmtime serve -Scli server.wasm

# Stdio
wasmtime run server.wasm
```

### MCP Server for AI-Assisted Development

The CLI includes a Model Context Protocol (MCP) server that provides AI assistants with tools and resources for wasmcp development.

#### Start the MCP server

```bash
# HTTP server on default port 8085
wasmcp mcp serve

# Custom port
wasmcp mcp serve --port 9000

# Stdio transport for local integration
wasmcp mcp serve --stdio

# Enable verbose logging
wasmcp mcp serve -v
```

#### Available Tools

The MCP server exposes the following tools:

- **compose** - Compose components into MCP servers with all CLI options (force, verbose, version, deps-dir, skip-download, override options)

#### Available Resources

The server provides read-only access to:

**Documentation** (fetched from GitHub):
- `wasmcp://docs/readme` - Project overview and quick start
- `wasmcp://docs/getting-started` - Step-by-step tutorial for first component
- `wasmcp://docs/cli` - Detailed CLI command documentation
- `wasmcp://docs/architecture` - Component model and composition pipeline
- `wasmcp://docs/examples` - Example components overview and learning path
- `wasmcp://docs/wit-interfaces` - Complete WIT interface documentation

**WIT Interfaces** (fetched from GitHub):
- `wasmcp://wit/protocol/mcp` - MCP protocol type definitions
- `wasmcp://wit/protocol/features` - Capability interfaces (tools, resources, prompts)
- `wasmcp://wit/server/handler` - Core handler interface for middleware
- `wasmcp://wit/server/sessions` - Session management interfaces
- `wasmcp://wit/server/notifications` - Notification interfaces

**Registry** (from local configuration):
- `wasmcp://registry/components` - Component aliases (JSON)
- `wasmcp://registry/profiles` - Composition profiles (JSON)
- `wasmcp://registry/config` - Full wasmcp.toml configuration

#### Integration

Configure MCP clients to connect to the server:

**Claude Desktop** (`claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "wasmcp": {
      "command": "wasmcp",
      "args": ["mcp", "serve", "--stdio"]
    }
  }
}
```

**HTTP Client**:
```bash
curl -X POST http://localhost:8085/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "id": 1, "method": "tools/list"}'
```

The MCP server feature is enabled by default. To build without it:

```bash
cargo build --no-default-features
```

## Architecture

### Capability Middleware Pattern

Components are built using the **capability pattern**, which separates business logic from protocol handling:

- **Capability components** (e.g., `tools-capability`) export clean, focused interfaces for specific functionality
- **Middleware components** (e.g., `tools-middleware`) handle MCP protocol translation and request delegation
- **CLI auto-detection** inspects components and automatically wraps capabilities with their middleware

This creates a separation of concerns where component authors focus on business logic, and the framework handles protocol complexity.

### Composition Pipeline

The CLI composes components into a linear pipeline:

```
stdio-transport
  ↓
tools-middleware (wraps math capability)
  ↓
tools-middleware (wraps strings capability)
  ↓
method-not-found
```

## Building Components

Scaffolded projects include Makefiles that handle dependencies automatically:

```bash
cd my-component
make  # Installs dependencies if needed, then builds the component
```

**Available targets:**
- `make` or `make build` - Build the component
- `make setup` - Explicitly install dependencies (optional, done automatically)
- `make clean` - Remove build artifacts

**Component output locations:**
- **Python**: `{project-name}.wasm` (in project root)
- **Rust**: `target/wasm32-wasip2/release/{project-name}.wasm`
- **TypeScript**: `dist/{project-name}.wasm`

See generated `README.md` files for language-specific details.

## Registry Configuration

### Location

Registry data is stored in:
- **Configuration file**: `~/.config/wasmcp/config.toml` (XDG Base Directory compliant)
- **Composed outputs**: `~/.config/wasmcp/composed/` (when using profiles without `-o` override)
- **Downloaded dependencies**: `~/.config/wasmcp/deps/`

**Output Path Behavior:**
- Explicit `-o` flag: Always uses current working directory (or absolute path if provided)
- Profile without `-o`: Uses `~/.config/wasmcp/composed/{profile-output}`
- No profile, no `-o`: Uses current working directory (`mcp-server.wasm`)

### Config File Format

The configuration uses TOML format:

```toml
# Component aliases
[components]
calc = "/absolute/path/to/calculator.wasm"
strings = "wasmcp:string-tools@1.0.0"
weather = "calc"  # Aliases can reference other aliases

# Profiles
[profiles.dev]
components = ["calc", "strings"]
output = "dev-server.wasm"

[profiles.prod]
base = "dev"  # Inherit from dev profile
components = ["monitor", "logger"]
output = "prod-server.wasm"
```

### Validation

The registry enforces:
- **Unique names**: Component aliases and profile names cannot conflict
- **No circular dependencies**: Detected in both alias chains and profile inheritance
- **Valid identifiers**: Names must be alphanumeric with hyphens/underscores only
- **Reserved names**: Cannot use CLI command names (compose, registry, etc.)

### View Registry

```bash
# Show all registry information
wasmcp registry info

# Filter to components only
wasmcp registry info --components
wasmcp registry info -c

# Filter to profiles only
wasmcp registry info --profiles
wasmcp registry info -p
```

## Version Compatibility

Components must use matching wasmcp versions. Specify version when creating components and composing:

```bash
wasmcp new my-component --language rust --version 0.4.0
wasmcp compose component.wasm --version 0.4.0
```

## See Also

- [WIT packages](../wit/)
- [Example Implementations](../examples/)
- [Component Model](https://github.com/WebAssembly/component-model)
- [MCP](https://spec.modelcontextprotocol.io/)
