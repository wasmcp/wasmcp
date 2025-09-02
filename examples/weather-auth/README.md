# Authorization-Enabled MCP Server Example

This example demonstrates how to compose an MCP server with OAuth 2.0 authorization using WebAssembly components.

## Architecture

```
[Weather Provider] + [Authorization Component] + [HTTP Transport] = [Secure MCP Server]
```

The composition happens in two steps:
1. Plug the authorization component into the HTTP transport
2. Plug the provider component into the auth-enabled transport

## Prerequisites

- `wac` - WebAssembly Compositor
- `wkg` - WebAssembly Package Manager
- `wasmtime` or `spin` - WebAssembly runtime

## Required Components

This example uses pre-published components from the registry. If they're not published yet, publish them first:

```bash
# 1. Publish auth-enabled HTTP transport
cd ../../components/http-transport
make publish-auth

# 2. Publish authorization component
cd ../authorization
make publish

# 3. Publish weather provider (Python example)
cd ../../examples/weather-py
make setup  # One-time setup for Python environment
make publish
```

## Quick Start

1. Build and compose all components:
```bash
make build
```

2. Set up environment variables (optional, for strict validation):
```bash
export MCP_EXPECTED_ISSUER=https://auth.example.com
export MCP_EXPECTED_AUDIENCE=https://mcp.example.com
```

3. Run the server:
```bash
make run
```

4. Test the authorization:
```bash
# Test without auth (should fail with 401)
make test-no-auth

# Test OAuth discovery endpoints
make test-discovery

# Generate a test token (development only)
make gen-test-token

# Test with token
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer YOUR_TOKEN_HERE" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":1}'
```

## Authorization Features

### OAuth 2.0 Discovery
The server provides standard OAuth discovery endpoints:
- `/.well-known/oauth-protected-resource` - Resource metadata (RFC 9728)
- `/.well-known/oauth-authorization-server` - Auth server metadata (RFC 8414)

### JWT Validation
- Signature verification (with JWKS support)
- Expiration and not-before checks
- Issuer and audience validation
- Scope-based authorization

### Policy-Based Access Control
The authorization component uses OPA/Rego policies to control access:
- Method-level authorization (tools/list, tools/call, etc.)
- Tool-specific permissions (dangerous tools require admin scope)
- Resource-level protection (sensitive resources require additional scopes)

## Configuration

### Environment Variables

```bash
# JWT Validation
export MCP_EXPECTED_ISSUER=https://auth.example.com
export MCP_EXPECTED_AUDIENCE=https://mcp.example.com
export MCP_JWKS_URI=https://auth.example.com/.well-known/jwks.json

# OAuth Discovery
export MCP_RESOURCE_URL=https://mcp.example.com
export MCP_AUTH_SERVER=https://auth.example.com
```

### Authorization Policies

The server uses the default policy from `components/authorization/policies/default.rego`.

Key policy rules:
- `tools/list` requires `mcp:tools:read` scope
- `tools/call` requires `mcp:tools:write` scope
- Dangerous tools require `admin` scope
- Resources require `mcp:resources:read` scope

## Development

### Generate Test Tokens

For development, you can generate test JWT tokens:

```bash
make gen-test-token
```

This creates a token with:
- Subject: test-user
- Scopes: mcp:tools:read, mcp:tools:write, mcp:resources:read
- Expiration: 1 hour

### Validate Components

Check that all components are built:

```bash
make validate-components
```

### View Policies

Examine the authorization policies:

```bash
make policy-default  # Default OAuth scope policy
make policy-rbac     # Role-based access control
make policy-tool     # Fine-grained tool authorization
```

## Security Notes

1. **Production Use**: Always use proper JWT signing keys and JWKS endpoints
2. **Token Storage**: Never log or store raw tokens
3. **Policy Updates**: Policies are embedded in the component - rebuild to update
4. **Scope Design**: Design scopes carefully for your security requirements
5. **HTTPS**: Always use HTTPS in production

## Troubleshooting

### 401 Unauthorized
- Check token is included: `Authorization: Bearer <token>`
- Verify token is not expired
- Check issuer and audience match configuration

### 403 Forbidden
- Verify token has required scopes
- Check policy allows the operation
- Review tool-specific requirements

### Component Build Failures
- Ensure WIT package is published: `wkg publish fastertools:mcp@0.1.11.wasm`
- Check Rust toolchain is up to date
- Verify cargo-component is installed

## Component Sizes

Typical component sizes:
- Authorization: ~1.7MB (includes Regorus policy engine)
- HTTP Transport: ~600KB
- Weather Provider: ~37MB (Python componentized)
- Final Server: ~39MB

## License

Apache-2.0