# {{project-name | kebab_case}}

{{project-description}}

## Structure

This is a Spin application that implements the Model Context Protocol (MCP) using WebAssembly components.

- `handler/` - The TypeScript implementation of your MCP handler
- `spin.toml` - Spin application manifest
- `Makefile` - Build and development commands

## Development

### Prerequisites

- Node.js >= 20.0.0
- Spin CLI
- componentize-js (will be installed automatically by Makefile)

### Building

```bash
make build
# or
spin build
```

### Testing

```bash
make test
```

### Running Locally

```bash
spin up
# or
make up
```

The MCP server will be available at `http://localhost:3000/mcp`

### Example Usage

```bash
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "{{project-name | snake_case}}",
      "arguments": {
        "input": "Hello, world!"
      }
    },
    "id": 1
  }'
```

### Example Usage

```bash
# List available tools
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "id": 1
  }'

# Call the echo tool
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "echo",
      "arguments": {
        "message": "Hello, world!"
      }
    },
    "id": 2
  }'
```

## Implementing Your Tool

Edit `handler/src/index.ts` to implement your tool's functionality:

1. Modify `listTools()` to define your tools
2. Implement the tool logic in `callTool()`
3. Optionally implement resources and prompts

## Type Safety

This template uses `jco` to generate TypeScript types from the WIT interface definition. The types are generated in `handler/src/generated/` when you run `npm run build`.

## Configuration

### Spin Configuration

Edit `spin.toml` to configure:
- Component source and version
- Environment variables
- Build commands

### Authentication (Optional)

By default, this MCP server runs without authentication. To add OAuth2/AuthKit authentication:

1. Replace the gateway component in `spin.toml`:
   ```toml
   [component.wasmcp-spin]
   # Instead of wasmcp-spin:
   # source = { registry = "ghcr.io", package = "fastertools:wasmcp-spin", version = "0.0.3" }
   
   # Use wasmcp-spin-authkit:
   source = { registry = "ghcr.io", package = "fastertools:wasmcp-spin-authkit", version = "0.1.0" }
   allowed_outbound_hosts = ["https://*"]  # Required for JWKS fetching
   
   [component.wasmcp-spin.variables]
   authkit_issuer = "https://your-company.authkit.app"
   authkit_jwks_uri = "https://your-company.authkit.app/oauth2/jwks"
   ```

2. All requests will now require a valid JWT token in the `Authorization: Bearer <token>` header

### Package Configuration

Edit `handler/package.json` to:
- Add dependencies
- Configure build scripts
- Update package metadata