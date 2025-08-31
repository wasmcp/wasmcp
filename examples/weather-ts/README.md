# weather-ts

An MCP server written in TypeScript

This is a TypeScript MCP (Model Context Protocol) capability provider that compiles to WebAssembly using the Component Model.

## Prerequisites

- Node.js 20 or later
- npm
- `wkg` and `wac` tools (installed by setup script)
- A WebAssembly runtime (Wasmtime or Spin)

## Quick Start

1. **Run setup** (installs dependencies and checks tools):
   ```bash
   make setup
   ```

2. **Build the component**:
   ```bash
   make build
   ```

3. **Run the server**:
   ```bash
   make run
   ```

The server will start on `http://localhost:8080/mcp`

## Development

### Project Structure

```
.
├── src/
│   ├── index.ts       # Main MCP provider implementation
│   ├── helpers.ts     # Helper functions for building tools
│   └── types.ts       # TypeScript type definitions
├── wit/
│   └── world.wit      # WebAssembly Interface Types
├── dist/              # Compiled JavaScript output
├── tsconfig.json      # TypeScript configuration
└── package.json       # Node.js dependencies
```

### Available Tools

This template includes three example tools:

1. **echo** - Echoes a message back
2. **get_weather** - Gets weather for a single location
3. **multi_weather** - Gets weather for multiple locations concurrently

### Building

The build process:
1. Compiles TypeScript to JavaScript (`tsc`)
2. Builds a WebAssembly component from the JavaScript (`jco componentize`)
3. Downloads a transport component from the registry
4. Composes the provider with the transport (`wac plug`)

```bash
make build
```

### Type Checking

Run TypeScript type checking without building:

```bash
make typecheck
```

### Testing

Test individual endpoints:

```bash
# Initialize the session
make test-init

# List available tools
make test-tools

# Test the echo tool
make test-echo

# Test the weather tool
make test-weather

# Test multi-weather tool
make test-multi
```

## Deployment

### With Wasmtime

```bash
wasmtime serve -Scli mcp-http-server.wasm
```

### With Spin

For deployment to Fermyon Cloud:

```bash
spin deploy
```

Or run locally:

```bash
spin up
```

## Adding New Tools

1. Define your tool's types in `src/types.ts`
2. Create a new tool using `createTool()` in `src/index.ts`
3. Add the tool to the `tools` array in `createHandler()`

Example:

```typescript
const myTool = createTool({
    name: 'my_tool',
    description: 'Description of what the tool does',
    schema: {
        type: 'object',
        properties: {
            param: { type: 'string', description: 'Parameter description' }
        },
        required: ['param']
    },
    execute: async (args) => {
        // Tool implementation
        return `Result: ${args.param}`;
    }
});
```

## Configuration

The MCP server endpoint is configured in `spin.toml` (default: `/mcp`).

Outbound HTTP hosts are configured in `spin.toml` under `allowed_outbound_hosts`.

## License

Apache-2.0