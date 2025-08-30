<div align="center">

# `wasmcp`

**MCP servers as WebAssembly components**

Run [Model Context Protocol](https://modelcontextprotocol.io) servers on [Spin](https://github.com/fermyon/spin), [Wasmtime](https://github.com/bytecodealliance/wasmtime), or any WASI runtime.

</div>

## Quick Start

Install templates
```bash
spin templates install --git https://github.com/fastertools/wasmcp --upgrade
```

Scaffold a new MCP server in your favorite source language
```bash
spin new -t wasmcp-rust my-weather-server --accept-defaults
cd my-weather-server
```

Build the handler component and compose with a server component
```bash
make build
make compose
```

Run with wasmtime (or spin up for Spin)
```bash
wasmtime serve -Scli composed.wasm

# Test it
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "method": "tools/list", "id": 1}'
```

That's it. Your composed.wasm server binary runs anywhere WebAssembly components do, or will.

## Examples

See [`examples/`](./examples/) for complete working servers.

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

## Components

The [`components/`](./components/) directory contains published components that are useful for composing MCP servers.

The `server-mcp-http-tools` component is published and publicly available at https://github.com/orgs/fastertools/packages/container/package/server-mcp-http-tools via `fastertools:server-mcp-http-tools@0.1.0`

## License

Apache-2.0