# Authentication Configuration

This MCP provider supports optional OAuth 2.0 authentication via JWT tokens.

## Quick Start (No Authentication)

By default, authentication is **disabled**. Simply run:

```bash
make build
make serve
```

The server will accept all requests without authentication.

## Enabling OAuth 2.0 Authentication

To enable authentication, edit `app.py` and modify the `get_auth_config()` function:

```python
def get_auth_config() -> Optional['authorization.ProviderAuthConfig']:
    if authorization:
        return authorization.ProviderAuthConfig(
            expected_issuer="https://your-auth-domain.example.com",
            expected_audiences=["your-client-id"],
            jwks_uri="https://your-auth-domain.example.com/oauth2/jwks",
            policy=None,  # Optional: Add Rego policy for additional authorization
            policy_data=None,  # Optional: Add policy data as JSON string
        )
```

Replace the placeholder values with your actual OAuth provider details:
- `expected_issuer`: Your OAuth issuer URL (e.g., AuthKit domain)
- `expected_audiences`: List of accepted audience values (typically your client ID)
- `jwks_uri`: URL to fetch public keys for JWT validation

Then rebuild:

```bash
make build
make serve
```

## How It Works

When authentication is enabled:
1. All MCP requests must include a `Bearer` token in the `Authorization` header
2. The token is validated against the configured issuer and audience
3. Invalid or missing tokens receive a `401 Unauthorized` response
4. The server provides OAuth discovery endpoints at:
   - `/.well-known/oauth-protected-resource` - For MCP clients to discover the auth server
   - `/.well-known/oauth-authorization-server` - For compatibility with legacy clients

## Using with AuthKit

If you're using WorkOS AuthKit:

1. Enable Dynamic Client Registration in your WorkOS Dashboard
2. Use your AuthKit domain as the issuer and in the JWKS URI
3. Use your client ID as the audience

Example:
```python
return authorization.ProviderAuthConfig(
    expected_issuer="https://your-app.authkit.app",
    expected_audiences=["client_YOUR_CLIENT_ID"],
    jwks_uri="https://your-app.authkit.app/oauth2/jwks",
    policy=None,
    policy_data=None,
)
```

## Custom Authorization Policies

You can add Rego policies for fine-grained authorization:

```python
return authorization.ProviderAuthConfig(
    # ... other config ...
    policy="""
        package mcp.authorization
        
        default allow = false
        
        allow {
            input.token.scopes[_] == "weather:read"
        }
    """,
    policy_data=None,
)
```

The policy receives:
- `input.token` - JWT claims (sub, iss, aud, scopes, etc.)
- `input.request` - HTTP request details (method, path, headers)
- `input.mcp` - MCP-specific context (method name, tool name, etc.)

## Testing Authentication

With authentication enabled, test with a bearer token:

```bash
# This will fail without a token
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":1}'

# This will work with a valid token
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":1}'
```