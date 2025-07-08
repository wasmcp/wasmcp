# wasmcp-spin-authkit

An authenticated MCP (Model Context Protocol) gateway component for Spin with built-in OAuth2/AuthKit support.

## Overview

`wasmcp-spin-authkit` is a WebAssembly component that provides OAuth2 authentication for MCP servers running on Spin. It combines the functionality of an HTTP gateway with JWT token validation, making it easy to protect your MCP tools and resources with WorkOS AuthKit or any OAuth2 provider.

## Features

- **OAuth2/JWT Authentication**: Validates JWT tokens from AuthKit or any OAuth2 provider
- **MCP Protocol Support**: Full support for tools, resources, and prompts (version 2025-03-26)
- **Metadata Endpoints**: OAuth2 discovery endpoints for MCP clients
- **User Context**: Includes authenticated user information in responses
- **Easy Integration**: Drop-in replacement for `wasmcp-spin`
- **Small binary size**: ~300KB

## Architecture

```
Client (with OAuth token) → wasmcp-spin-authkit → Your MCP Handler
                                    ↓
                              JWT Validation
```

## Usage

### 1. Replace the standard gateway in your `spin.toml`:

```toml
[[trigger.http]]
route = "/..."
component = "mcp-server"

[component.mcp-server]
# Instead of wasmcp-spin:
# source = { registry = "ghcr.io", package = "fastertools:wasmcp-spin", version = "0.0.3" }

# Use wasmcp-spin-authkit:
source = { registry = "ghcr.io", package = "fastertools:wasmcp-spin-authkit", version = "0.1.0" }
allowed_outbound_hosts = ["https://*"]  # Required for JWKS fetching

[component.mcp-server.variables]
authkit_issuer = "https://your-company.authkit.app"
authkit_jwks_uri = "https://your-company.authkit.app/oauth2/jwks"
# authkit_audience = "your-audience"  # Optional

[component.mcp-server.dependencies]
"wasmcp:mcp/handler" = { path = "./handler/target/wasm32-wasip1/release/handler.wasm" }
```

### 2. Configure your AuthKit settings:

The component requires these environment variables:
- `authkit_issuer`: Your AuthKit domain (e.g., `https://your-company.authkit.app`)
- `authkit_jwks_uri`: JWKS endpoint for token validation
- `authkit_audience` (optional): Expected audience claim in tokens

### 3. OAuth2 Discovery

The component exposes OAuth2 metadata endpoint for MCP client discovery:
- `/.well-known/oauth-protected-resource`: Returns resource metadata

## Authentication Flow

1. **Client obtains OAuth token** from AuthKit
2. **Client sends request** with `Authorization: Bearer <token>` header
3. **Component validates token**:
   - Verifies issuer matches configuration
   - Checks audience (if configured)
   - Validates expiration
4. **On success**: Request forwarded to MCP handler with user context
5. **On failure**: Returns 401 with WWW-Authenticate header

## Example Response with User Context

When authenticated, the initialize response includes user information:

```json
{
  "protocolVersion": "2025-03-26",
  "capabilities": {
    "tools": {},
    "resources": {},
    "prompts": {}
  },
  "serverInfo": {
    "name": "wasmcp-spin-authkit",
    "version": "0.1.0",
    "authInfo": {
      "authenticated_user": "user_123",
      "email": "user@example.com"
    }
  }
}
```

## Building from Source

```bash
cd src/components/wasmcp-spin-authkit
cargo component build --release
```

## Configuration Examples

### Basic AuthKit Setup
```toml
[component.mcp-server.variables]
authkit_issuer = "https://divine-lion-50.authkit.app"
authkit_jwks_uri = "https://divine-lion-50.authkit.app/oauth2/jwks"
```

### With Audience Validation
```toml
[component.mcp-server.variables]
authkit_issuer = "https://divine-lion-50.authkit.app"
authkit_jwks_uri = "https://divine-lion-50.authkit.app/oauth2/jwks"
authkit_audience = "https://api.myapp.com"
```

## Error Responses

### Missing Authorization
```
HTTP/1.1 401 Unauthorized
WWW-Authenticate: Bearer error="unauthorized", error_description="Missing authorization header"
Content-Type: application/json

{
  "error": "unauthorized",
  "error_description": "Missing authorization header"
}
```

### Invalid Token
```
HTTP/1.1 401 Unauthorized
WWW-Authenticate: Bearer error="unauthorized", error_description="Invalid token format"
Content-Type: application/json

{
  "error": "unauthorized",
  "error_description": "Invalid token format"
}
```

## Security Considerations

- Always use HTTPS in production
- Keep your AuthKit configuration secure
- Regularly rotate signing keys
- Monitor authentication failures

## Differences from wasmcp-spin

| Feature | wasmcp-spin | wasmcp-spin-authkit |
|---------|-------------|---------------------|
| Authentication | None | OAuth2/JWT |
| User Context | No | Yes |
| Metadata Endpoints | No | Yes |
| Network Access | Not required | Required (for JWKS) |

## License

Apache-2.0