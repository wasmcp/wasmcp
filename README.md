<div align="center">

# `wasmcp`

**Build MCP servers on the WebAssembly Component Model**
</div>

This project represents the [Model Context Protocol 2025-06-18+](https://modelcontextprotocol.io/specification/2025-06-18) spec in the [WIT](https://component-model.bytecodealliance.org/design/wit.html) (Wasm Interface Type) language.

These types enable you to build transport components in one language, and plug in the capabilities implementations from components authored in other languages.

The results is a single `.wasm` binary file that works on any runtime that supports the [WebAssembly Component Model](https://wasmcloud.com/). Some examples are [Wasmtime](https://github.com/bytecodealliance/wasmtime), [Spin](https://github.com/fermyon/spin), [wasmCloud](https://wasmcloud.com/) or the many emerging platforms and runtimes that are adopting this standard.

The component model is a broad-reaching architecture for building interoperable WebAssembly libraries, applications, and environments.

## Quick Start

Try running one of the example servers in your favorite source language.
```bash
cd examples/weather-py
```

Ensure build dependencies are set up. The [examples/](./examples/) depend only on [wkg](https://github.com/bytecodealliance/wasm-pkg-tools) (for WIT package management), [wac](https://github.com/bytecodealliance/wac) (for component composition), and the standard toolchain of your chosen source language. You can install these manually, or run:
```bash
make setup
```

Build the handler component
```bash
make build
```

Compose the handler component with a transport component
```bash
make compose
```

That's it. Your `composed.wasm` server binary runs anywhere WebAssembly components do, or will.

Try it out with a runtime that supports Wasm components, like [Wasmtime](https://github.com/bytecodealliance/wasmtime)
```bash
wasmtime serve -Scli composed.wasm
```

Use the running MCP server in a compatible client
```json
{
  "mcpServers": {
    "wasmTools": {
      "url": "http://localhost:8080/mcp",
      "transport": "http"
    }
  }
}
```

```bash
claude mcp add -t http wasmTools http://localhost:8080/mcp
```

## Spin

Spin users can run Wasm components out-of-the-box.

```bash
spin up --from composed.wasm
```

These components also work with Spin v3's built-in [component dependencies](https://spinframework.dev/v3/writing-apps#using-component-dependencies) feature, where you might specify a transport component as a Spin http component, and plug in a handler component to satisfy its capabilities dependencies.

You can install the examples in this repo as templates, to scaffold new MCP handler components in different source languages.
```bash
spin templates install --git https://github.com/fastertools/wasmcp --upgrade
```

The resulting structure will include a `spin.toml` file that lets you deploy with:
```bash
spin deploy
```

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