# Auth Demo - MCP Server with AuthKit

This example demonstrates how to build an authenticated MCP server using `wasmcp-spin-authkit` with WorkOS AuthKit for OAuth2 authentication.

## Architecture

```
┌─────────────┐     ┌───────────────────┐     ┌─────────────┐
│  MCP Client │────▶│ wasmcp-spin-authkit│────▶│   Handler   │
└─────────────┘     └───────────────────┘     └─────────────┘
                             │
                             ▼
                        AuthKit/OAuth2
```

The architecture uses a simple two-component pattern:
1. **Gateway Component** (`wasmcp-spin-authkit`): Handles HTTP, OAuth2 authentication, and MCP protocol
2. **Handler Component**: Your MCP implementation with tools, resources, and prompts

## Quick Start

### 1. Build the Handler

```bash
cd handler
cargo component build --release
cd ..
```

### 2. Configure AuthKit

Update `spin-authkit.toml` with your AuthKit configuration:
```toml
[component.mcp-server.variables]
authkit_issuer = "https://your-app.authkit.app"
authkit_jwks_uri = "https://your-app.authkit.app/oauth2/jwks"
# authkit_audience = "your-audience"  # Optional
```

### 3. Run the Server

```bash
spin up -f spin-authkit.toml
```

### 4. Test Authentication

```bash
# Without auth (should fail with 401)
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":1}'

# Check OAuth metadata
curl http://localhost:3000/.well-known/oauth-protected-resource

# With a valid token (obtain from AuthKit)
curl -X POST http://localhost:3000/mcp \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":1}'
```

## Components

### Handler (`handler/`)
Your MCP implementation with:
- **Tools**: `echo` and `user_info`
- **Resources**: `file:///readme`
- **Prompts**: `welcome`

### Gateway Component

The `wasmcp-spin-authkit` gateway provides:
- OAuth2/JWT authentication with AuthKit
- MCP protocol compliance (version 2025-03-26)
- User context in responses
- OAuth metadata endpoints for client discovery
- Automatic 401 responses with proper WWW-Authenticate headers

## Configuration

The example includes two Spin configurations:

### `spin.toml` - Development (No Auth)
Uses the standard `wasmcp-spin` gateway for local development without authentication.

### `spin-authkit.toml` - Production (With Auth)
Uses `wasmcp-spin-authkit` with AuthKit configuration:

```toml
[component.mcp-server]
source = "../../src/components/wasmcp-spin-authkit/target/wasm32-wasip1/release/wasmcp_spin_authkit.wasm"
allowed_outbound_hosts = ["https://*"]  # For JWKS fetching

[component.mcp-server.variables]
authkit_issuer = "https://divine-lion-50-staging.authkit.app"
authkit_jwks_uri = "https://divine-lion-50-staging.authkit.app/oauth2/jwks"

[component.mcp-server.dependencies]
"wasmcp:mcp/handler" = { path = "./handler/target/wasm32-wasip1/release/auth_demo_handler.wasm" }
```

## MCP Handler Features

The example handler demonstrates:

### Tools
- `echo` - Returns the provided message
- `user_info` - Returns information about the authenticated user

### Resources
- `file:///readme` - Provides this README content

### Prompts
- `welcome` - A personalized welcome message

## Testing with Claude Desktop

1. Configure Claude Desktop to use your authenticated MCP server
2. Ensure Claude Desktop has OAuth2 support enabled
3. The client will handle the OAuth flow automatically

## Next Steps

1. **Set up your own AuthKit domain** at https://workos.com
2. **Update the configuration** in `spin-authkit.toml`
3. **Implement your MCP tools** in the handler
4. **Deploy to Spin Cloud** for production use

## Resources

- [wasmcp Documentation](https://github.com/fastertools/wasmcp)
- [AuthKit Documentation](https://workos.com/docs/authkit)
- [MCP Specification](https://modelcontextprotocol.io)
- [Spin Framework](https://spin.fermyon.dev)