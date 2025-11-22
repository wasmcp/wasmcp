# wasmcp CLI

Build composable MCP servers using WebAssembly components.

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/wasmcp/wasmcp/main/install.sh | bash
```

Installs to `~/.wasmcp/bin` and configures PATH. For a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/wasmcp/wasmcp/main/install.sh | bash -s -- --version 0.4.4
```

Alternatively, download binaries from [releases](https://github.com/wasmcp/wasmcp/releases) or build from source with `cargo build --release`.

Requires a runtime like [Wasmtime](https://wasmtime.dev/) to run composed servers.

## Usage

### Create a new component

```bash
wasmcp new my-math --language rust # --type tools by default
wasmcp new my-strings -l python -t resources # MCP resources
```

Generated projects include simple component implementations.

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

Components can be specified in multiple formats:

#### Component Specification Formats

**Registry Packages (OCI):**
```bash
# Format: namespace:name[@version]
# The colon (:) identifies it as a registry package

wasmcp compose server wasmcp:calculator@0.1.0 -o server.wasm    # With version (recommended)
wasmcp compose server wasmcp:calculator -o server.wasm          # Latest version
wasmcp compose server namespace:handler@2.0.0 -o server.wasm   # Custom namespace
```

Registry packages are downloaded from OCI registries (e.g., `ghcr.io/wasmcp`) and cached in `~/.config/wasmcp/deps/`.

**Local File Paths:**
```bash
# Detected by: starts with ./ ../ ~/ / or contains / or ends with .wasm

wasmcp compose server ./component.wasm -o server.wasm           # Relative path
wasmcp compose server ../target/handler.wasm -o server.wasm    # Parent directory
wasmcp compose server /abs/path/component.wasm -o server.wasm  # Absolute path
wasmcp compose server ~/projects/handler.wasm -o server.wasm   # Home directory
wasmcp compose server handler.wasm -o server.wasm              # Current directory
```

**Aliases:**
```bash
# Registered in ~/.config/wasmcp/config.toml

wasmcp compose server calc strings -o server.wasm              # Using aliases
```

**Profiles:**
```bash
# Expand to multiple components in-place

wasmcp compose server dev                                       # Profile (uses profile output)
wasmcp compose server dev -o custom.wasm                        # Override profile output
```

**Mixed Formats:**
```bash
# You can mix any combination

wasmcp compose server dev calc ./local.wasm wasmcp:remote@1.0 -o server.wasm
```

#### Transport Options

```bash
wasmcp compose server component.wasm -t stdio -o server.wasm   # Stdio transport
wasmcp compose server component.wasm -t http -o server.wasm    # HTTP transport (default)
```

#### Advanced Options

```bash
# Force overwrite existing output
wasmcp compose server calc strings -o server.wasm --force

# Verbose output for debugging resolution
wasmcp compose server calc strings -o server.wasm --verbose

# Custom dependency directory
wasmcp compose server calc --deps-dir ./my-deps --skip-download

# Override framework components (paths or versions)
wasmcp compose server calc --override transport=./custom-transport.wasm
wasmcp compose server calc --override method-not-found=0.2.0
wasmcp compose server calc --override server-io=https://example.com/server-io.wasm
```

**Resolution order:** profile → alias → path → registry package

The CLI automatically detects component types and wraps them with appropriate middleware.

### Run the server

```bash
# HTTP (default)
wasmtime serve -Scli server.wasm

# Stdio
wasmtime run server.wasm
```

### MCP Server for AI-Assisted Development

The CLI includes a Model Context Protocol (MCP) server that provides AI assistants with tools and resources for wasmcp development.

**Quick Start:**

```bash
# Foreground mode (development)
wasmcp mcp serve

# Background daemon (production)
wasmcp mcp start
wasmcp mcp status
wasmcp mcp logs
wasmcp mcp stop
```

See **[Daemon Management Guide](../docs/daemon-management.md)** for complete documentation on running the server as a background daemon.

#### Start the MCP server (foreground)

```bash
# HTTP server on default port 8085
wasmcp mcp serve

# Custom port
wasmcp mcp serve --port 9000

# Stdio transport for local integration
wasmcp mcp serve --stdio

# Enable verbose logging
wasmcp mcp serve -v

# Use local filesystem instead of GitHub for resources (development only)
wasmcp mcp serve --local-resources /path/to/wasmcp
```

#### Available Tools

The MCP server exposes the following tools:

- **compose** - Compose components into MCP servers with all CLI options
- **registry_list** - List registry components, profiles, and aliases
- **registry_add_component** - Add a component alias to the registry
- **registry_add_profile** - Add or update a composition profile
- **registry_remove** - Remove a component alias or profile

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
- `wasmcp://wit/mcp` - Complete MCP protocol types and capability interfaces
- `wasmcp://wit/server` - Server interfaces (handler and messages)
- `wasmcp://wit/sessions` - Session management interfaces

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

The MCP server uses the Streamable HTTP transport with Server-Sent Events (SSE). For testing, use an MCP client like [Claude Desktop](https://claude.ai/download) or the [MCP Inspector](https://github.com/modelcontextprotocol/inspector).

For programmatic access, use an MCP client library that supports the streamable HTTP transport (see [MCP specification](https://spec.modelcontextprotocol.io/specification/2025-03-26/basic/transports#streamable-http)).

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

Component versions are managed through `versions.toml` in the CLI. This file specifies the exact version of each framework component (transport, server-io, authorization, middleware components, etc.).

### Override Framework Components

Use the `--override` flag to customize framework components or versions during composition. The flag accepts both local paths (ending in `.wasm`) and version strings:

```bash
# Override with a custom component (local path)
wasmcp compose server component.wasm --override transport=./custom-transport.wasm

# Override with a specific version
wasmcp compose server component.wasm --override transport=0.2.0

# Override with a remote URL
wasmcp compose server component.wasm --override server-io=https://example.com/server-io.wasm

# Override multiple components (mix paths and versions)
wasmcp compose server component.wasm \
  --override transport=./custom-transport.wasm \
  --override tools-middleware=0.2.0 \
  --override authorization=0.1.1
```

**Valid component names:**
- `transport` - HTTP or stdio transport layer
- `server-io` - Server I/O interface implementation
- `authorization` - Authorization/authentication handler
- `kv-store` - Key-value storage interface
- `session-store` - Session management
- `method-not-found` - Terminal handler for unknown methods
- `tools-middleware` - Tools capability middleware
- `resources-middleware` - Resources capability middleware
- `prompts-middleware` - Prompts capability middleware

## See Also

- **[Daemon Management Guide](../docs/daemon-management.md)** - Running the MCP server as a background daemon
- [WIT packages](../wit/) - Interface definitions
- [Example Implementations](../examples/) - Sample components
- [Component Model](https://github.com/WebAssembly/component-model) - WebAssembly Component specification
- [MCP Specification](https://spec.modelcontextprotocol.io/) - Model Context Protocol
