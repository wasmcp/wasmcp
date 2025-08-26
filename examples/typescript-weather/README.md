# typescript-weather

An MCP server written in TypeScript

## Structure

This is a Spin application that implements the Model Context Protocol (MCP) using WebAssembly components.

- `handler/` - The TypeScript implementation of your MCP handler
- `spin.toml` - Spin application manifest
- `Makefile` - Build and development commands

## Development

### Prerequisites

- Node.js >= 20.0.0
- Spin CLI
- @bytecodealliance/jco (will be installed automatically)

### Building

```bash
# Build the handler component
make build

# Compose with gateway
make compose
```

### Testing

The handler includes comprehensive unit tests for all tools:

```bash
make test
```

Tests cover:
- Tool metadata (name, description)
- Input schema validation
- Successful execution paths
- Error handling for invalid inputs

### Running Locally

#### With Wasmtime (standalone WASI runtime)
```bash
wasmtime serve -S cli -S http composed.wasm
```

The MCP server will be available at `http://localhost:8080`

#### With Spin
```bash
spin up
```

The MCP server will be available at `http://localhost:3000/mcp`

### Example Usage

```bash
# List available tools
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "id": 1
  }'

# Call the weather tool
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "weather",
      "arguments": {
        "location": "San Francisco"
      }
    },
    "id": 2
  }'
```

## Implementing Your Tools

Edit `handler/src/index.ts` to add new tools:

```typescript
import { createTool, createHandler, z } from 'wasmcp';

const myTool = createTool({
  name: 'my_tool',
  description: 'Description of my tool',
  schema: z.object({
    param: z.string().describe('Parameter description')
  }),
  execute: async (args) => {
    // Your tool logic here
    return `Result for ${args.param}`;
  }
});

// Add to handler
export const handler = createHandler({
  tools: [echoTool, weatherTool, myTool]
});
```

## Configuration

### Spin Configuration

Edit `spin.toml` to configure:
- Component source and version
- Environment variables
- Build commands

### Package Configuration

Edit `handler/package.json` to:
- Add dependencies
- Configure build scripts
- Update package metadata