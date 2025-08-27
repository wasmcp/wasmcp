# {{project-name | title_case}}

{{project-description}}

## Architecture

This MCP handler uses the **compositional pattern** - it only implements the capabilities it needs (tools by default), and uses WebAssembly composition to fill in the rest.

```
Your Handler + Null Components + Server = Complete MCP Server
```

- **Your Handler**: Implements only what you need (tools, resources, or prompts)
- **Null Components**: Provide empty implementations for missing capabilities
- **WAC Composition**: Wires everything together into a complete server

## Structure

```
.
├── src/
│   ├── lib.rs          # Your handler implementation (tools only by default)
│   └── bindings.rs     # Auto-generated from WIT
├── wit/
│   └── world.wit       # Declares what your handler exports
├── compose.wac         # Composition script that wires components
├── Cargo.toml          # Rust dependencies
├── Makefile            # Build and test automation
└── server.wasm         # Pre-built MCP server component
```

## Quick Start

```bash
# Build and compose everything
make compose

# Run the server
make run

# In another terminal, test it
make test-init       # Initialize
make test-tools      # List tools
make test-resources  # List resources (empty from null component)
```

## Implementing Your Handler

### 1. Tools Only (Default)

The template starts with tools only. Implement your tools in `src/lib.rs`:

```rust
fn handle_list_tools(...) -> Result<ListToolsResponse, McpError> {
    // Return your tool definitions
}

fn handle_call_tool(request: CallToolRequest) -> Result<ToolResult, McpError> {
    // Handle tool calls
}
```

### 2. Adding Resources

To add resource support:

1. Update `wit/world.wit`:
```wit
export fastertools:mcp/resource-handler@0.1.1;
```

2. Implement the trait in `src/lib.rs`:
```rust
impl bindings::exports::fastertools::mcp::resource_handler::Guest for Component {
    // ... implement resource methods
}
```

3. Update `compose.wac` to use your handler's resources instead of null:
```wac
"fastertools:mcp/resource-handler@0.1.1": handler["fastertools:mcp/resource-handler@0.1.1"],
```

### 3. Adding Prompts

Similar process - update WIT, implement trait, update composition.

## How Composition Works

The `compose.wac` file defines how components are wired together:

1. **Instantiate components** with implicit imports (`...`)
2. **Wire exports** from handlers to server inputs
3. **Export** the server's HTTP handler

WAC automatically handles:
- Type imports from the registry
- Dependency resolution
- Component validation

## Testing

```bash
# Test individual capabilities
make test-init       # Server info
make test-tools      # Your tools
make test-resources  # Empty (null component)
make test-prompts    # Empty (null component)

# Test your specific tools
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"example_tool","arguments":"{\"message\":\"test\"}"},"id":1}'
```

## Deployment

The `composed.wasm` file is a complete MCP server that can be deployed anywhere WebAssembly runs:
- Spin
- Wasmtime
- WasmEdge
- Docker with Wasm support
- Cloud platforms with Wasm support

## Benefits of This Approach

1. **Focused Development**: Only implement what you need
2. **Type Safety**: Only get bindings for what you implement
3. **Modularity**: Mix and match different capability providers
4. **Small Size**: Handlers are tiny (~100KB), composition adds what's needed
5. **No SDK Required**: Direct bindings, no abstraction layers

## Troubleshooting

- **"Package not found" errors**: Make sure null components are built or downloaded
- **Composition fails**: Check that all components are built with `make build build-nulls`
- **Server doesn't start**: Ensure port 8080 is free
- **Tools not showing**: Check your tool definitions return valid JSON schemas

## Learn More

- [MCP Specification](https://modelcontextprotocol.io)
- [WebAssembly Component Model](https://component-model.bytecodealliance.org/)
- [WAC Language](https://github.com/bytecodealliance/wac)