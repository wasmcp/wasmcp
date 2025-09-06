<div align="center">

# `wasmcp`

**Build MCP servers on the [WebAssembly Component Model](https://component-model.bytecodealliance.org/)**
</div>

[wit/](./wit/) expresses the [Model Context Protocol](https://modelcontextprotocol.io/specification/2025-06-18) specification in the [WIT](https://component-model.bytecodealliance.org/design/wit.html) (WebAssembly Interface Type) language.

These [published types](https://github.com/orgs/fastertools/packages/container/package/mcp) enable polyglot MCP implementations via WebAssembly components. Transport components can be written once and reused with capability providers in any language.

The composition process (`provider + transport = mcp-http-server.wasm`) produces a standalone MCP server that runs on any component model runtime: [Wasmtime](https://github.com/bytecodealliance/wasmtime), [Spin](https://github.com/spinframework/spin), [wasmCloud](https://github.com/wasmCloud/wasmCloud), and others.

## Quick start

Try running one of the example servers in your favorite source language. All examples provide transparent implementations that use WIT bindings directly as the SDK.

```bash
cd examples/weather-py    # Python
cd examples/weather-go    # Go
cd examples/weather-rs    # Rust
cd examples/weather-ts    # TypeScript
cd examples/weather-js    # JavaScript
```

Ensure build dependencies are set up. The [examples/](./examples/) depend only on [wkg](https://github.com/bytecodealliance/wasm-pkg-tools) (for WIT package management), [wac](https://github.com/bytecodealliance/wac) (for component composition), and the standard toolchain of your chosen source language. Run setup to check and install these tools:
```bash
make setup
```

Run any language-specific setup steps
```bash
source venv/bin/activate  # Python
```

Build and compose the capability provider with a transport component
```bash
make build
```

That's it. Your `mcp-http-server.wasm` server binary runs anywhere WebAssembly components do, or will.

Try it out with a runtime that supports Wasm components, like [Wasmtime](https://github.com/bytecodealliance/wasmtime)
```bash
wasmtime serve -Scli mcp-http-server.wasm
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

Spin users can run Wasm components out of the box.

```bash
spin up --from mcp-http-server.wasm
```

These components also work with Spin v3's built-in [component dependencies](https://spinframework.dev/v3/writing-apps#using-component-dependencies) feature, where you might specify a transport component as a Spin http component, and plug in a provider component to satisfy its capabilities dependencies.

You can install the templates in this repo to scaffold new MCP provider components in different source languages.
```bash
spin templates install --git https://github.com/fastertools/wasmcp --upgrade
```

Create a new MCP server project:
```bash
spin new -t wasmcp-python my-mcp-server    # Python
spin new -t wasmcp-go my-mcp-server         # Go
spin new -t wasmcp-rust my-mcp-server       # Rust
spin new -t wasmcp-typescript my-mcp-server # TypeScript
spin new -t wasmcp-javascript my-mcp-server # JavaScript
```

The resulting structure will include a `spin.toml` file that you can use for composing, running, and deploying components.
```bash
spin cloud deploy
```
```
View application:   https://weather-py-xxxxxxxx.fermyon.app/
  Routes:
  - mcp-server: https://weather-py-xxxxxxxx.fermyon.app/mcp
```

## Examples

See [`examples/`](./examples/) for complete working servers implementing tools capabilities. Each example provides a transparent implementation that uses WIT bindings directly as the SDK.

```python
# Python example using direct WIT bindings
class WeatherMCPCapabilities(ToolsCapabilities, CoreCapabilities):
    """Direct implementation of the MCP capabilities interfaces."""
    
    def handle_initialize(self, request: InitializeRequest) -> InitializeResponse:
        return InitializeResponse(
            protocol_version="v20250618",
            capabilities=ServerCapabilities(tools=ToolsCapability()),
            server_info=ImplementationInfo(
                name="weather-py",
                version="0.1.0",
                title="weather-py Server"
            ),
            instructions="A Python MCP server providing weather tools"
        )
    
    def handle_call_tool(self, request: CallToolRequest) -> ToolResult:
        if request.name == "echo":
            args = json.loads(request.arguments or "{}")
            return text_result(f"Echo: {args.get('message', '')}")
        # ... other tools
```

## WIT

The Wasm Interface Type ([WIT](https://component-model.bytecodealliance.org/design/wit.html)) package in [`wit/`](./wit/) aims to capture a complete representation of the MCP specification. It currently reflects the 2025-06-18 version of the spec, with some additional elements from the latest draft.

The WIT package is published as Wasm at https://github.com/orgs/fastertools/packages/container/package/mcp. It can be fetched with `wkg wit fetch` when included as a dependency in a component's world:

```wit
// world.wit
package weather-js:provider;

// MCP tools for a JavaScript provider
world weather-js {
    import wasi:http/outgoing-handler@0.2.3;
    export fastertools:mcp/core-capabilities@0.4.0;
    export fastertools:mcp/tools-capabilities@0.4.0;
}
```

A capability provider component does not necessarily depend on I/O. It can be a pure computational component that can run in browsers, embedded systems, or any WebAssembly hosts - it just exports functions that transform MCP requests to responses.

A provider with I/O, directly for outbound HTTP or indirectly via composition with an HTTP transport component, uses the WebAssembly System Interface ([WASI](https://github.com/WebAssembly/WASI)) to interact with the outside world.

## Components

The [`components/`](./components/) directory contains published components that are useful for composing MCP servers.

The HTTP transport component is published and publicly available at https://github.com/orgs/fastertools/packages/container/package/mcp-transport-http-tools via `fastertools:mcp-transport-http-tools@0.4.2`. This transport provides:
- JSON-RPC over HTTP
- Built-in OAuth 2.0 authentication support
- JWKS caching capabilities
- Rego policy enforcement (optional)

## Why components?

From https://component-model.bytecodealliance.org/design/why-component-model.html#benefits-of-the-component-model

>Moreover, a component interacts with a runtime or other components only by calling its imports and having its exports called. Specifically, unlike core modules, a component may not export a memory and thus it cannot indirectly communicate to others by writing to its memory and having others read from that memory. This not only reinforces sandboxing, but enables interoperation between languages that make different assumptions about memory: for example, allowing a component that relies on garbage-collected memory to interoperate with one that uses conventional linear memory.

## License

Apache-2.0