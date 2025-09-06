# MCP HTTP Transport Component

Implements the Streamable HTTP transport for MCP in Rust, including spec-compliant authorization and optional policy-based authorization via [OPA/Rego(rus)](https://github.com/microsoft/regorus).

Supported MCP capababilities are configured by feature flags corresponding to the transport worlds in [mcp.wit](/wit/mcp.wit). This allows capability provider components to only implement what they need to, while avoiding deadcode bloat from unused capabilities. An alternative is to use stub/null components to fill in the gaps for a full transport world.

The default, tools-only transport component is published and publicly available at https://github.com/orgs/fastertools/packages/container/package/mcp-transport-http-tools via `fastertools:mcp-transport-http-tools@0.1.11`+

### Composition

```bash
wac plug --plug capabilities-provider.wasm transport.wasm -o server.wasm
```

## Authorization

The HTTP transport component provides comprehensive OAuth 2.0 authorization and policy-based authorization capabilities:

### Architecture

Authorization is **optional** and controlled by the provider component via `get_auth_config()`. When enabled:

1. **JWT Validation** (`jwt.rs`): Validates bearer tokens from Authorization headers
   - Fetches JWKS from configured endpoint with 1-hour caching
   - Validates token signature, issuer, audience, expiry
   - Extracts claims including `sub`, `scopes`, `client_id`
   - Supports RSA and HMAC algorithms

2. **Policy Engine** (`policy.rs`): Evaluates Rego policies for fine-grained access control
   - Uses `regorus` for OPA-compatible policy evaluation
   - Receives token claims, request context, and MCP method details
   - Returns allow/deny decisions with optional denial reasons

3. **OAuth Discovery** (`discovery.rs`): Exposes standard discovery endpoints
   - `/.well-known/oauth-protected-resource`: Resource metadata per RFC 8414
   - `/.well-known/oauth-authorization-server`: Server metadata for client configuration

### Request Flow

```
Request → Check Auth Config → Validate JWT → Apply Policy → Route to Handler
              ↓ (if disabled)                    ↓ (if denied)
         Direct routing                    401/403 Response
```

### Provider Configuration

Providers supply auth configuration through `ProviderAuthConfig`:
- `expected_issuer`: Required issuer claim value
- `expected_audiences`: List of accepted audience values  
- `jwks_uri`: JWKS endpoint for key discovery
- `policy`: Optional Rego policy for authorization rules
- `policy_data`: Optional static data for policy evaluation

### Policy Context

Policies receive comprehensive context in `policy.rs:parse_mcp_context()`:
```json
{
  "token": { "sub", "iss", "aud", "scopes", "client_id" },
  "request": { "method", "path", "headers" },
  "mcp": { "method", "tool", "arguments" }
}
```

### Error Handling

- **401 Unauthorized**: Invalid/expired token, missing auth header
- **403 Forbidden**: Policy denial, insufficient scopes
- Includes WWW-Authenticate header with error details

### Features

- JWKS caching reduces latency and load on auth servers
- Clock skew tolerance (60s) for distributed systems
- Support for multiple audiences
- Context-aware authorization based on MCP operations

## License

Apache-2.0