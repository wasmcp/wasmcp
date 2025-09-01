# MCP Authorization Component

A production-ready WebAssembly component providing OAuth 2.0 authorization, JWT validation, and policy-based access control for MCP servers.

## Features

- **JWT Validation**: Full JWT token validation with JWKS support
- **OAuth 2.0 Compliance**: Implements RFC 8414 (Authorization Server Metadata) and RFC 9728 (Protected Resource Metadata)
- **Policy Engine**: OPA/Rego policy evaluation using Regorus
- **MCP-Aware**: Fine-grained authorization for MCP methods, tools, and resources
- **Component Model**: Pure WebAssembly component with WIT interfaces

## Architecture

This component provides authorization as a pluggable capability that can be composed with MCP transport components:

```
[HTTP Request] 
    → [http-transport with auth]
        → [authorization component]
            ├── [JWT validation]
            ├── [Policy evaluation]
            └── [OAuth discovery]
        → [MCP provider component]
    → [HTTP Response]
```

## WIT Interfaces

The component exports five main interfaces:

### 1. Authorization Interface
Main authorization function that validates tokens and applies policies.

```wit
authorize: func(request: auth-request) -> auth-response;
```

### 2. JWT Validator Interface
Standalone JWT validation with JWKS support.

```wit
validate: func(request: jwt-request) -> jwt-result;
fetch-jwks: func(uri: string) -> result<string, string>;
```

### 3. Policy Engine Interface
OPA/Rego policy evaluation for fine-grained access control.

```wit
evaluate: func(request: policy-request) -> policy-result;
```

### 4. OAuth Discovery Interface
OAuth 2.0 discovery endpoints for client configuration.

```wit
get-resource-metadata: func() -> resource-metadata;
get-server-metadata: func() -> server-metadata;
```

### 5. MCP Authorization Interface
MCP-specific authorization helpers for methods, tools, and resources.

```wit
authorize-method: func(request: mcp-auth-request) -> result<_, auth-error>;
authorize-tool: func(request: tool-auth-request) -> result<_, auth-error>;
authorize-resource: func(request: resource-auth-request) -> result<_, auth-error>;
```

## Configuration

The component can be configured through environment variables:

```bash
# OAuth/JWT Configuration
MCP_EXPECTED_ISSUER=https://auth.example.com
MCP_EXPECTED_AUDIENCE=https://mcp.example.com
MCP_JWKS_URI=https://auth.example.com/.well-known/jwks.json

# Discovery Configuration
MCP_RESOURCE_URL=https://mcp.example.com
MCP_AUTH_SERVER=https://auth.example.com
MCP_AUTH_ENDPOINT=https://auth.example.com/authorize
MCP_TOKEN_ENDPOINT=https://auth.example.com/token
MCP_REGISTRATION_ENDPOINT=https://auth.example.com/register
```

## Policy Examples

### Default Policy
The default policy (`policies/default.rego`) implements basic OAuth scope-based authorization:

```rego
# Allow if user has required scope for method
allow if {
    input.token.sub != ""
    method_allowed
}

method_allowed if {
    input.mcp.method == "tools/call"
    "mcp:tools:write" in input.token.scopes
    tool_allowed
}
```

### RBAC Policy
The RBAC policy (`policies/rbac.rego`) implements role-based access control:

```rego
allow if {
    user_roles := get_user_roles(input.token.sub)
    required_permission := get_required_permission
    some role in user_roles
    required_permission in data.roles[role].permissions
}
```

### Tool Authorization Policy
The tool authorization policy (`policies/tool-authorization.rego`) provides fine-grained control over tool access:

```rego
tool_authorized if {
    "mcp:tools:write" in input.token.scopes
    tool_name := input.mcp.tool
    tool_check_passes(tool_name)
}
```

## Integration with HTTP Transport

To use this component with the HTTP transport, enable the `auth` feature:

```toml
[dependencies.mcp-transport-http]
features = ["auth", "tools", "resources"]
```

The transport will automatically:
1. Handle OAuth discovery endpoints (`/.well-known/*`)
2. Validate bearer tokens on incoming requests
3. Apply configured policies
4. Return proper OAuth error responses

## Building

Build the component:

```bash
cd components/authorization
cargo component build --release
```

The resulting WebAssembly component will be at `target/wasm32-wasip1/release/mcp_authorization.wasm`.

## Testing

Run the test suite:

```bash
cargo test
```

Test policies:

```bash
# Test with the OPA CLI
opa test policies/*.rego -v
```

## Security Considerations

1. **Token Validation**: Always validates JWT signatures, expiration, issuer, and audience
2. **No Token Passthrough**: Authorization context is passed between components, never raw tokens
3. **Policy Isolation**: Each policy evaluation runs in isolation with controlled input
4. **Secure Defaults**: Defaults to deny unless explicitly allowed by policy
5. **Audit Logging**: Policies can generate audit logs for compliance

## OAuth 2.0 Compliance

This component implements:
- RFC 6749: OAuth 2.0 Authorization Framework
- RFC 8414: OAuth 2.0 Authorization Server Metadata
- RFC 9728: OAuth 2.0 Protected Resource Metadata
- RFC 8707: Resource Indicators for OAuth 2.0
- OAuth 2.1 Draft: Enhanced security requirements

## Performance

- **JWT Caching**: JWKS are cached for 1 hour to reduce network calls
- **Policy Compilation**: Rego policies are compiled once and reused
- **Zero Network Overhead**: Authorization happens in-process via component calls
- **Minimal Allocations**: Efficient memory usage with minimal copying

## License

Apache-2.0