# MCP HTTP Transport Component

HTTP transport component for MCP servers. Handles JSON-RPC over HTTP and composes with capability providers.

## Features

Build variants based on required capabilities:
- `tools` - Tool providers
- `resources` - Resource providers
- `prompts` - Prompt providers
- `auth` - OAuth 2.0 authorization

## Usage

### From Registry

```bash
# Tools only
wkg get fastertools:mcp-transport-http-tools@0.1.0 -o transport.wasm

# Tools with auth
wkg get fastertools:mcp-transport-http-tools-auth@0.1.0 -o transport.wasm
```

### Composition

```bash
wac plug --plug provider.wasm transport.wasm -o server.wasm
```

## Building

```bash
# Tools only
cargo component build --release --no-default-features --features tools

# Tools with auth
cargo component build --release --no-default-features --features "tools auth"
```

## Published Packages

- `fastertools:mcp-transport-http-tools@0.1.0`
- `fastertools:mcp-transport-http-tools-auth@0.1.0`

## Implementation

Written in Rust using:
- `spin-sdk` for HTTP handling
- `rmcp` for JSON-RPC processing
- WASI for runtime compatibility

## Size

~550KB per variant

## Authentication & Authorization

The HTTP transport component provides comprehensive OAuth 2.0 authentication and policy-based authorization capabilities:

### Architecture

Authentication is **optional** and controlled by the provider component via `get_auth_config()`. When enabled:

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