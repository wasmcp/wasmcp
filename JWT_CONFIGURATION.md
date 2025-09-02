# JWT Configuration Guide for wasmcp Authorization

This guide explains how to configure JWT validation for your MCP server with OAuth 2.0 authorization.

## Configuration Methods

### Method 1: Using Environment Variables (Spin)

Since Spin doesn't support WASI config yet, use environment variables:

```bash
JWT_ISSUER="https://your-auth.auth0.com/" \
JWT_AUDIENCE="https://your-api-identifier" \
JWT_JWKS_URI="https://your-auth.auth0.com/.well-known/jwks.json" \
spin up --from test-auth-server.wasm --listen 127.0.0.1:3000
```

### Method 2: Using WASI Config (Wasmtime)

Create a config file and run with wasmtime:

```bash
wasmtime run \
  --wasi config=wasmtime-runner.toml \
  --tcplisten 127.0.0.1:3000 \
  test-auth-server.wasm
```

## Configuration Parameters

### Required JWT Settings

| Config Key | Environment Variable | Description | Example |
|------------|---------------------|-------------|---------|
| `jwt.expected_issuer` | `JWT_ISSUER` | The expected issuer of JWT tokens | `https://auth0-domain.auth0.com/` |
| `jwt.expected_audience` | `JWT_AUDIENCE` | The expected audience for tokens | `https://your-api-identifier` |
| `jwt.jwks_uri` | `JWT_JWKS_URI` | URI to fetch JWKS for signature validation | `https://auth0-domain.auth0.com/.well-known/jwks.json` |

### Optional JWT Settings

| Config Key | Default | Description |
|------------|---------|-------------|
| `jwt.validation_leeway` | 60 | Clock skew tolerance in seconds |

### OAuth Discovery Settings

These configure the OAuth discovery endpoints that clients can use:

| Config Key | Description |
|------------|-------------|
| `oauth.resource_url` | Your MCP server's URL |
| `oauth.auth_server` | Authorization server URL |
| `oauth.auth_issuer` | OAuth issuer identifier |
| `oauth.auth_endpoint` | Authorization endpoint |
| `oauth.token_endpoint` | Token endpoint |
| `oauth.registration_endpoint` | Dynamic client registration endpoint (optional) |

### Policy Settings

| Config Key | Options | Description |
|------------|---------|-------------|
| `policy.mode` | `default`, `rbac`, `custom`, `none` | Authorization policy mode |
| `policy.path` | File path | Path to custom Rego policy (when mode=custom) |

## Provider-Specific Examples

### Auth0

1. Create an API in Auth0 Dashboard
2. Note your domain and API identifier
3. Configure:

```toml
[[wasi.config]]
"jwt.expected_issuer" = "https://dev-abc123.auth0.com/"
"jwt.expected_audience" = "https://my-mcp-api"
"jwt.jwks_uri" = "https://dev-abc123.auth0.com/.well-known/jwks.json"
```

### Google Identity Platform

1. Create OAuth 2.0 credentials in Google Cloud Console
2. Configure:

```toml
[[wasi.config]]
"jwt.expected_issuer" = "https://accounts.google.com"
"jwt.expected_audience" = "YOUR_CLIENT_ID.apps.googleusercontent.com"
"jwt.jwks_uri" = "https://www.googleapis.com/oauth2/v3/certs"
```

### Azure AD / Microsoft Entra ID

1. Register an app in Azure Portal
2. Configure:

```toml
[[wasi.config]]
"jwt.expected_issuer" = "https://login.microsoftonline.com/YOUR_TENANT_ID/v2.0"
"jwt.expected_audience" = "api://YOUR_CLIENT_ID"
"jwt.jwks_uri" = "https://login.microsoftonline.com/YOUR_TENANT_ID/discovery/v2.0/keys"
```

## Testing Your Configuration

### 1. Check Discovery Endpoints (No Auth Required)

```bash
# Resource metadata
curl http://localhost:3000/.well-known/oauth-protected-resource

# Authorization server metadata
curl http://localhost:3000/.well-known/oauth-authorization-server
```

### 2. Test Without Token (Should Fail)

```bash
curl -X POST http://localhost:3000 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":1}'
```

Expected: 401 Unauthorized with `WWW-Authenticate` header

### 3. Test With Valid Token

First, obtain a valid JWT token from your OAuth provider, then:

```bash
curl -X POST http://localhost:3000 \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":1}'
```

Expected: 200 OK with MCP response

## Obtaining Test Tokens

### Auth0
Use the Auth0 Dashboard test feature or:
```bash
curl -X POST https://YOUR_DOMAIN.auth0.com/oauth/token \
  -H "Content-Type: application/json" \
  -d '{
    "client_id": "YOUR_CLIENT_ID",
    "client_secret": "YOUR_CLIENT_SECRET",
    "audience": "YOUR_API_IDENTIFIER",
    "grant_type": "client_credentials"
  }'
```

### Google
Use OAuth 2.0 Playground: https://developers.google.com/oauthplayground/

### Azure AD
Use Azure CLI:
```bash
az account get-access-token --resource api://YOUR_CLIENT_ID
```

## Troubleshooting

### Common Issues

1. **"Invalid issuer"** - Check that `jwt.expected_issuer` matches the `iss` claim in your token exactly (including trailing slashes)

2. **"Invalid audience"** - Ensure `jwt.expected_audience` matches the `aud` claim in your token

3. **"Invalid signature"** - Verify the JWKS URI is correct and accessible

4. **"Token expired"** - Check token expiration and system clock; adjust `jwt.validation_leeway` if needed

### Debug Mode

Enable debug output by setting:
```bash
RUST_LOG=debug spin up --from test-auth-server.wasm
```

## Policy Modes

### Default Mode
Allows any authenticated user:
```rego
allow {
    input.token.sub != ""
}
```

### RBAC Mode
Checks for specific scopes:
```rego
allow {
    input.token.scopes[_] == "admin"
}

allow {
    input.mcp.method == "tools/list"
    input.token.scopes[_] == "read"
}
```

### Custom Mode
Load your own Rego policy from `policy.path`

### None Mode
No policy evaluation (JWT validation only)

## Next Steps

1. Set up your OAuth provider
2. Configure the appropriate settings
3. Test with the discovery endpoints
4. Obtain a test token
5. Make authenticated MCP requests

For production deployments, consider:
- Using a secrets manager for sensitive configuration
- Implementing token refresh logic in clients
- Setting up proper CORS headers
- Monitoring and logging authentication failures
- Rate limiting authentication attempts