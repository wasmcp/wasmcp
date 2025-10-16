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

### Compose components into a server

```bash
# Single component
wasmcp compose component.wasm -o server.wasm

# Multiple components (composed into linear pipeline)
wasmcp compose math.wasm strings.wasm -o server.wasm

# Stdio transport for local integration
wasmcp compose component.wasm -t stdio -o server.wasm
```

The CLI automatically detects component types and wraps them with appropriate middleware.

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
