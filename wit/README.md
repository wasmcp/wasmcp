## WIT

The Wasm Interface Type ([WIT](https://component-model.bytecodealliance.org/design/wit.html)) package in this directory provides a complete representation of the MCP specification. It reflects the 2025-06-18 version of the spec, with additional elements from the latest draft.

The WIT package is published as Wasm at https://github.com/orgs/fastertools/packages/container/package/mcp. It can be fetched with `wkg wit fetch` when included as a dependency in a component's world:

```wit
// world.wit
package weather-js:provider;

// MCP provider component written in JavaScript
world weather-js {
    // Import WASI HTTP for outbound requests (optional)
    import wasi:http/outgoing-handler@0.2.3;
    
    // Export MCP capabilities
    export fastertools:mcp/core-capabilities@0.4.0;
    export fastertools:mcp/tools-capabilities@0.4.0;
}
```

## Capabilities

The WIT package defines the following MCP capabilities:

- **`core-capabilities`**: Session management (initialize, ping, shutdown) and optional authentication configuration
- **`tools-capabilities`**: Tool listing and execution
- **`prompts-capabilities`**: Prompt templates (future)
- **`resources-capabilities`**: Resource access and subscriptions (future)
- **`completions-capabilities`**: Completion suggestions (future)
- **`logging-capabilities`**: Structured logging (future)

## Authentication

The `core-capabilities` interface includes optional OAuth 2.0 authentication support:

```wit
get-auth-config: func() -> option<provider-auth-config>
jwks-cache-get: func(jwks-uri: string) -> option<string>
jwks-cache-set: func(jwks-uri: string, jwks: string)
```

Providers can return authentication configuration to enable OAuth 2.0 protection with JWT validation, JWKS caching, and optional Rego policy enforcement.

## Component Architecture

A capability provider component does not necessarily depend on I/O. An MCP provider can be a pure computational component that can run in browsers, embedded systems, or any WebAssembly host - it just exports functions that transform MCP requests to responses.

A provider with I/O, directly for outbound HTTP or indirectly via composition with an HTTP transport component, uses the WebAssembly System Interface ([WASI](https://github.com/WebAssembly/WASI)) to interact with the outside world.

The composition process (`provider + transport = mcp-http-server.wasm`) produces a standard WASI component that runs directly on any compliant runtime.