# mcp-weather-demo

An MCP tool written in TypeScript

## Quick Start

After creating this project with `spin new`, you can immediately run your MCP server:

```bash
# First, build the handler component
make build

# Then run with Spin (handles dependencies automatically)
spin up

# Or run with wasmtime (requires manual composition)
make run-wasmtime
```

## Available Commands

```bash
make help           # Show all available commands
make build          # Build the handler component
make compose        # Compose handler with gateway
make run            # Run with Spin
make run-wasmtime   # Run with wasmtime
make test-tools     # Test the tools/list endpoint
make test-echo      # Test the echo tool
```

## Project Structure

```
.
├── handler/               # Your MCP handler implementation
│   ├── src/
│   │   └── index.ts      # Handler code with tools, resources, and prompts
│   ├── wit/
│   │   └── mcp.wit       # MCP interface definition
│   └── package.json      # Node.js dependencies
├── composed.wasm         # Final composed component (after `make compose`)
└── Makefile             # Build and run commands
```

## Development Workflow

1. **Edit your handler**: Modify `handler/src/index.ts` to add tools, resources, or prompts
2. **Build and compose**: Run `make compose` to build and compose your component
3. **Test locally**: Run `make run` to start the server
4. **Test your tools**: Use `make test-tools` and `make test-echo` to test

## Adding New Tools

Edit `handler/src/index.ts`:

```typescript
import { createTool, z } from 'wasmcp';

export const myTool = createTool({
  name: 'my_tool',
  description: 'Description of what my tool does',
  schema: z.object({
    param: z.string().describe('Parameter description')
  }),
  execute: async (args) => {
    // Your tool implementation
    return `Result: ${args.param}`;
  }
});

// Don't forget to add it to the tools array
export const tools = [echoTool, myTool];
```

## Testing Your MCP Server

Once running, test your MCP server with curl:

```bash
# List available tools
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":1}'

# Call a tool
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":"{\"message\":\"Hello!\"}"},"id":2}'
```

## Runtime Options

This MCP server can run on multiple runtimes:

- **Spin**: Full-featured runtime with KV store support
- **Wasmtime**: Lightweight WASI runtime
- **Any WASI runtime**: The composed component is runtime-agnostic

## Requirements

- Node.js 20+
- Spin CLI (for `spin up`)
- wasmtime (for `wasmtime serve`)
- wac (for component composition)
- wkg (optional, for downloading gateway from registry)

## Learn More

- [MCP Documentation](https://modelcontextprotocol.io)
- [WebAssembly Component Model](https://component-model.bytecodealliance.org)
- [Spin Documentation](https://developer.fermyon.com/spin)