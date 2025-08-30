## WIT

The Wasm Interface Type ([WIT](https://component-model.bytecodealliance.org/design/wit.html)) package in [`wit/`](./wit/) aims to capture a complete representation of the MCP specification. It currently reflects the 2025-06-18 version of the spec, with some additional elements from the latest draft.

The WIT package is published as Wasm at https://github.com/orgs/fastertools/packages/container/package/mcp. It can be fetched with `wkg wit fetch` when included as a dependency in a component's world:

```
/// world.wit
package weather-js:handler;

/// MCP tools for An MCP server written in JavaScript
world weather-js {
    export fastertools:mcp/tool-handler@0.1.9;
}
```

A handler component does not necessarily depend on I/O. An MCP handler can be a pure computational component that can run in browsers, embedded systems, or any WebAssembly host - it just exports functions that transform MCP requests to responses.

A handler with I/O (directly for outbound HTTP or indirectly via composition with an HTTP server component) uses the WebAssembly System Interface ([WASI](https://github.com/WebAssembly/WASI)) to interact with the outside world.

The composition process (`handler + server = composed.wasm`) produces a standard WASI component that runs directly on any compliant runtime.