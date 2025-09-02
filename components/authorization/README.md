# MCP Authorization Component

A WebAssembly component providing OAuth 2.0 authorization and JWT validation for MCP servers.

## Features

- JWT validation with JWKS support
- OPA/Rego policy evaluation using Regorus
- OAuth 2.0 discovery endpoints (RFC 8414, RFC 9728)
- MCP-aware authorization for methods, tools, and resources
- WASI config runtime configuration

## Architecture

Authorization happens through component composition:

```
authorization.wasm + transport.wasm → transport-with-auth.wasm
provider.wasm + transport-with-auth.wasm → mcp-server.wasm
```

## Usage

### From Registry

```bash
wkg get fastertools:mcp-authorization@0.1.0 -o auth.wasm
wkg get fastertools:mcp-transport-http-tools-auth@0.1.0 -o transport.wasm

wac plug --plug auth.wasm transport.wasm -o transport-auth.wasm
wac plug --plug provider.wasm transport-auth.wasm -o server.wasm
```

### Configuration

Runtime configuration via WASI config:

```bash
wasmtime serve -Scli -Sconfig \
  -Sconfig-var="jwt.expected_issuer=https://auth.example.com" \
  -Sconfig-var="jwt.expected_audience=client_123" \
  -Sconfig-var="jwt.jwks_uri=https://auth.example.com/.well-known/jwks.json" \
  server.wasm
```

### Policy Modes

- `default` - Permissive, allows authenticated users
- `rbac` - Role-based access control with scope requirements  
- `custom` - User-provided OPA/Rego policy
- `none` - Skip policy evaluation

## Authorization Flow

1. **JWT Validation**
   - Fetch JWKS from issuer (cached for 1 hour)
   - Verify signature (RS256/HS256)
   - Validate claims (iss, aud, exp, nbf)

2. **Policy Evaluation**
   - Extract MCP context from request
   - Evaluate OPA/Rego policy
   - Return allow/deny decision

## WIT Interfaces

- `authorization` - Main authorization interface
- `jwt-validator` - Standalone JWT validation
- `policy-engine` - OPA/Rego evaluation
- `oauth-discovery` - Discovery endpoints
- `mcp-authorization` - MCP-specific helpers

## Building

```bash
cargo component build --release
```

## Testing

```bash
cargo test
```

## Size

~1.8MB including Regorus policy engine

## License

Apache-2.0