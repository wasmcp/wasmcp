## WIT

The Wasm Interface Type ([WIT](https://component-model.bytecodealliance.org/design/wit.html)) package in [`wit/`](./wit/) aims to capture a complete representation of the MCP specification. It currently reflects the 2025-06-18 version of the spec, with some additional elements from the latest draft.

The WIT package is published as Wasm at https://github.com/orgs/fastertools/packages/container/package/mcp. It can be fetched with `wkg wit fetch` when included as a dependency in a component's world:

```
/// world.wit
package weather-js:provider;

/// MCP tools for an MCP provider written in JavaScript
world weather-js {
    export fastertools:mcp/tools-capabilities@0.1.10;
}
```

A capability provider component does not necessarily depend on I/O. An MCP provider can be a pure computational component that can run in browsers, embedded systems, or any WebAssembly host - it just exports functions that transform MCP requests to responses.

A provider with I/O, directly for outbound HTTP or indirectly via composition with an HTTP transport component, uses the WebAssembly System Interface ([WASI](https://github.com/WebAssembly/WASI)) to interact with the outside world.

The composition process (`provider + transport = mcp-http-server.wasm`) produces a standard WASI component that runs directly on any compliant runtime.