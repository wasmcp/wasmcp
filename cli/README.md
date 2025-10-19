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

# Stdio transport for local integration
wasmcp compose component.wasm -t stdio -o server.wasm

# Verbose output for debugging
wasmcp compose calc strings -o server.wasm --verbose
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

## Architecture

### Capability Middleware Pattern

Components are built using the **capability pattern**, which separates business logic from protocol handling:

- **Capability components** (e.g., `tools-capability`) export clean, focused interfaces for specific functionality
- **Middleware components** (e.g., `tools-middleware`) handle MCP protocol translation and request delegation
- **CLI auto-detection** inspects components and automatically wraps capabilities with their middleware

This creates a clean separation of concerns where component authors focus on business logic, and the framework handles protocol complexity.

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

**Auto-detection in action:**
1. CLI inspects each component's WIT exports using wasmparser
2. Components exporting `wasmcp:mcp/tools-capability` are automatically wrapped with `tools-middleware`
3. Wrapped components expose the universal `wasmcp:mcp/server-handler` interface
4. All components form a uniform pipeline where requests flow downstream until handled

**Benefits:**
- **Simple composition**: Just list your components, no configuration needed
- **Automatic merging**: Middleware handles tool catalog merging automatically
- **Clean code**: Capability components have no delegation or merging logic
- **Type safety**: WIT interfaces ensure correct wiring at composition time

### Example: Tools Capability

A tools capability component only implements two methods:

```rust
fn list_tools(request: ListToolsRequest, client: ClientContext) -> ListToolsResult {
    // Return your tools
}

fn call_tool(request: CallToolRequest, client: ClientContext) -> Option<CallToolResult> {
    // Return Some(result) if you handle this tool, None otherwise
}
```

No protocol handling, no delegation, no merging. The middleware handles everything else.

## Building Components

Generated projects include Makefiles that handle dependencies automatically:

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
- **Composed outputs**: `~/.config/wasmcp/composed/` (when profiles specify relative paths)
- **Downloaded dependencies**: `~/.config/wasmcp/deps/`

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

- [WIT Interface Reference](../wit/)
- [Example Implementations](../examples/)
- [Component Model](https://github.com/WebAssembly/component-model)
- [MCP Protocol](https://spec.modelcontextprotocol.io/)
