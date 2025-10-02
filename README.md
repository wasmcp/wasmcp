<div align="center">

# `wasmcp`

A [WebAssembly Component](https://component-model.bytecodealliance.org/) Development Kit for [Model Context Protocol](https://modelcontextprotocol.io/docs/getting-started/intro) servers

</div>

## Quick Start

See [Installation](#installation)

```bash
# Scaffold a new handler
wasmcp new my-tools --type tools --language rust

# Compose handlers with an HTTP transport and middleware
wasmcp compose \
  --middleware ./auth.wasm \
  --tools ./my-tools.wasm \
  --middleware ./logging.wasm \
  --resources ./my-resources.wasm \
  -o server.wasm

# Serve a complete MCP server over HTTP
wasmtime serve -Scommon server.wasm

# Or compose with stdio transport
wasmcp compose \
  --tools ./my-tools.wasm \
  --transport stdio \
  -o server-stdio.wasm

wasmtime run server-stdio.wasm
```

See [cli/README.md](cli/README.md) and [examples/hello-world](examples/hello-world/) for complete examples in Rust, Python, TypeScript, and Go.

## Why?

> [!TIP]
> You only write the handlers for the individual MCP features and middlewares that you need.

Wasmcp provides [WIT](https://component-model.bytecodealliance.org/design/wit.html) (Wasm Interface Type) definitions and [published](https://github.com/orgs/wasmcp/packages) framework components for building complete, deployable MCP servers as WebAssembly components.

Any language with a [component toolchain](https://component-model.bytecodealliance.org/language-support.html) can be used for any individual component in the server.

MCP servers are:
- **Modular** - Composed of discrete capabilities (tools, resources, prompts, etc.) that can be implemented progressively
- **Security-sensitive** - Handling client requests in a least-privilege sandbox is a core requirement of secure MCP servers
- **Performance-sensitive** - Scalability and efficiency dictate the types of clients that can be served. Real-time AI applications require real-time tool responses

WebAssembly components are
- **Efficient** - Run portably on a wide variety of hosts, including edge workers
- **Secure** - Execute in a least-privilege sandbox
- **Lean** - Published framework components are each under 300KB
- **Composable** - Compose multiple components together into new ones, like binary lego bricks

They are a natural fit.

## Components

### Handler components (you implement as needed)

- **tools-handler** - Handles `tools/list` and `tools/call` methods
- **resources-handler** - Handles `resources/read` method
- **prompts-handler** - Handles `prompts/get` method
- **completion-handler** - Handles `completion/complete` method
- **middleware** - Any custom middleware (e.g. logging, auth), as many as needed, at any point in the chain

### Framework components (published to ghcr.io/wasmcp)

- **[http-transport](./crates/http-transport/)** - HTTP server transport using WASI HTTP (for `wasmtime serve`)
- **[stdio-transport](./crates/stdio-transport/)** - Stdio transport
- **[request](./crates/request/)** - Parses MCP JSON-RPC requests and manages request context
- **[initialize-writer](./crates/initialize-writer/)** - Formats MCP initialization responses
- **[tools-writer](./crates/tools-writer/)** - Formats tool execution results
- **[resources-writer](./crates/resources-writer/)** - Formats resource content responses
- **[initialize-handler](./crates/initialize-handler/)** - Terminal handler for initialization requests

## Installation

Download the latest release binary for your platform from [GitHub Releases](https://github.com/wasmcp/wasmcp/releases):


**Linux (x86_64):**
```bash
curl -fsSL https://github.com/wasmcp/wasmcp/releases/latest/download/wasmcp-x86_64-unknown-linux-gnu.tar.gz | tar -xz
sudo mv wasmcp /usr/local/bin/
```

**Linux (ARM64):**
```bash
curl -fsSL https://github.com/wasmcp/wasmcp/releases/latest/download/wasmcp-aarch64-unknown-linux-gnu.tar.gz | tar -xz
sudo mv wasmcp /usr/local/bin/
```

**macOS (Apple Silicon):**
```bash
curl -fsSL https://github.com/wasmcp/wasmcp/releases/latest/download/wasmcp-aarch64-apple-darwin.tar.gz | tar -xz
sudo mv wasmcp /usr/local/bin/
```

**macOS (Intel):**
```bash
curl -fsSL https://github.com/wasmcp/wasmcp/releases/latest/download/wasmcp-x86_64-apple-darwin.tar.gz | tar -xz
sudo mv wasmcp /usr/local/bin/
```

**Verify installation:**
```bash
wasmcp --version
```

**Or build from source:**
```bash
cargo install --git https://github.com/wasmcp/wasmcp wasmcp
```

## Architecture

Wasmcp prescribes a [chain-of-responsibility](https://en.wikipedia.org/wiki/Chain-of-responsibility_pattern) pattern where components are composed into request processing pipelines. A typical chain:

```
Transport → Middleware/Handlers -> Terminus
```

**Transport components** terminate the transport protocol (HTTP or stdio) and pass the JSON-RPC request and `wasi:io/streams.{output-stream}` to the next component. They import the `incoming-handler` interface and export transport-specific interfaces:
- **HTTP transport** - Exports `wasi:http/incoming-handler` for use with `wasmtime serve`
- **Stdio transport** - Exports `wasi:cli/run` and uses newline-delimited JSON-RPC over stdin/stdout per the MCP specification

**Middleware components** intercept requests to add capabilities, perform authorization, logging, or other cross-cutting concerns. They both import and export the `incoming-handler` interface. As many middleware components as needed can be chained together and interleaved with other components in the chain.

**Handler components** are middleware that process specific MCP methods (`tools/call`, `resources/read`, `prompts/get`). They terminate the response for their supported method, short-circuiting the chain. Unrecognized methods are forwarded to the next component in the chain. Handlers can be interleaved with generic middleware in any order.

**`initialize` handler** processes the `initialize` method using capabilities accumulated from upstream handler components. It acts as the terminal handler, completing the chain and handling any remaining unprocessed requests.

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
