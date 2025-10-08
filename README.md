<div align="center">

# `wasmcp`

A [WebAssembly Component](https://component-model.bytecodealliance.org/) Development Kit for [Model Context Protocol](https://modelcontextprotocol.io/docs/getting-started/intro) servers

</div>

## Quick Start

See [Installation](#installation)

```bash
# Start a new handler
wasmcp new my-tools --type tools --language python

# Develop your handler
cd my-tools && source venv/bin/activate && code app.py

# Compile it to a component binary
make

# Compose your component binary with the default HTTP transport component
wasmcp compose --tools target/my_tools.wasm -o http_mcp_server.wasm

# Run the composed MCP server over HTTP
wasmtime serve -Scommon http_mcp_server.wasm

# Or stdio
wasmcp compose --tools target/my_tools.wasm --transport stdio -o stdio_mcp_server.wasm
wasmtime run stdio_mcp_server.wasm
```

See [cli/README.md](cli/README.md) and [examples/hello-world](examples/hello-world/) for complete examples in Rust, Python, TypeScript, and Go.

## Why?

> [!TIP]
> You only write handlers for the individual MCP server features and middleware that you need.

WebAssembly components work server-side. They are
- **Composable** - Compose compiled component binaries together into new ones, like legos.
- **Sandboxed** - A component [interacts](https://component-model.bytecodealliance.org/design/why-component-model.html#benefits-of-the-component-model) with a runtime or other components only by calling its imports and having its exports called.
- **Distributable** - Push and pull component binaries from OCI registries.
- **Lean** - Fully composed servers can be under 1MB.

These qualities complement MCP's server [architecture](https://modelcontextprotocol.io/specification/2025-06-18/architecture).

## What's here?

Wasmcp provides a framework for for building complete, deployable MCP servers as WebAssembly components.

*You* author specific MCP server features and middleware in your favorite language, as a WebAssembly component.

Your component binary can then be composed with a set of [published](https://github.com/orgs/wasmcp/packages?repo_name=wasmcp) binaries that collectively implement the rest of the server. Any of the default framework components can be swapped out during composition for another that fulfills its [WIT world](https://component-model.bytecodealliance.org/design/worlds.html#wit-worlds).

Any language with a [component toolchain](https://component-model.bytecodealliance.org/language-support.html) can be used for any individual component in the server.

### Handler components (Implement what you need now, add features progressively)

- **tools-handler** - Handles `tools/list` and `tools/call` methods
- **resources-handler** - Handles `resources/read` method
- **prompts-handler** - Handles `prompts/get` method
- **completion-handler** - Handles `completion/complete` method
- **middleware** - Any custom middleware (e.g. logging, auth), as many as needed, at any point in the chain

### Framework components (Published to ghcr.io/wasmcp)

- **[http-transport](./crates/http-transport/)** - HTTP server transport using WASI HTTP (for `wasmtime serve`)
- **[stdio-transport](./crates/stdio-transport/)** - Stdio transport
- **[request](./crates/request/)** - Parses MCP JSON-RPC requests and manages request context
- **[initialize-writer](./crates/initialize-writer/)** - Formats MCP initialization responses
- **[tools-writer](./crates/tools-writer/)** - Formats tool execution results
- **[resources-writer](./crates/resources-writer/)** - Formats resource content responses
- **[initialize-handler](./crates/initialize-handler/)** - Terminal handler for initialization requests

## Installation

**Build from source:**
```bash
cargo install --git https://github.com/wasmcp/wasmcp
```

Or download the latest release binary for your platform from [GitHub Releases](https://github.com/wasmcp/wasmcp/releases):


**Linux (x86_64):**
```bash
curl -fsSL https://github.com/wasmcp/wasmcp/releases/latest/download/wasmcp-x86_64-unknown-linux-gnu.tar.gz | tar -xz
```

**Linux (ARM64):**
```bash
curl -fsSL https://github.com/wasmcp/wasmcp/releases/latest/download/wasmcp-aarch64-unknown-linux-gnu.tar.gz | tar -xz
```

**macOS (Apple Silicon):**
```bash
curl -fsSL https://github.com/wasmcp/wasmcp/releases/latest/download/wasmcp-aarch64-apple-darwin.tar.gz | tar -xz
```

**macOS (Intel):**
```bash
curl -fsSL https://github.com/wasmcp/wasmcp/releases/latest/download/wasmcp-x86_64-apple-darwin.tar.gz | tar -xz
```

---

**Move the binary to a directory in your PATH:**
```bash
sudo mv wasmcp /usr/local/bin/
```

**Verify installation:**
```bash
wasmcp --version
```

## Architecture

Wasmcp prescribes a [chain-of-responsibility](https://en.wikipedia.org/wiki/Chain-of-responsibility_pattern) pattern where components are composed into a JSON-RPC call processing pipeline. A typical chain:

```
transport → middleware/handlers -> terminal handler
```

**Transport components** terminate the transport protocol (HTTP or stdio) and pass a handle to the JSON-RPC object and `wasi:io/streams.{output-stream}` to the next component. They import the `incoming-handler` interface and export transport-specific interfaces:
- **HTTP transport** - Exports `wasi:http/incoming-handler` for use with `wasmtime serve`
- **Stdio transport** - Exports `wasi:cli/run` and uses newline-delimited JSON-RPC over stdin/stdout per the MCP specification

**Middleware components** intercept JSON-RPC calls to add capabilities, perform authorization, logging, or other cross-cutting concerns. They both import and export the `incoming-handler` interface. As many middleware components as needed can be chained together and interleaved with other components in the chain.

**Handler components** are middleware that process specific MCP methods (`tools/call`, `resources/read`, `prompts/get`). They terminate the response for their supported method, short-circuiting the chain. Unrecognized methods are forwarded to the next component in the chain. Handlers can be interleaved with generic middleware in any order.

**`initialize` handler** processes the `initialize` method using capabilities accumulated from upstream handler components. It acts as the terminal handler, completing the chain.

Components communicate in memory through WIT interfaces defined in the [/wit](/wit) package. Composition happens with [wac](https://github.com/bytecodealliance/wac).

## Examples

**hello-world/** - Minimal echo tool server in four languages
- Python, Rust, TypeScript, Go
- Same functionality, different implementations

**polyglot-composition/** - Multi-language polyglot server
- TypeScript + Go middlewares
- Rust MCP tools
- Python MCP resources
- Demonstrates composition patterns

## License

Apache 2.0
