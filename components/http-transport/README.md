# MCP HTTP Transport Component

HTTP transport component for MCP servers. Handles JSON-RPC over HTTP and composes with capability providers.

## Features

Build variants based on required capabilities:
- `tools` - Tool providers
- `resources` - Resource providers
- `prompts` - Prompt providers
- `auth` - OAuth 2.0 authorization

## Usage

### From Registry

```bash
# Tools only
wkg get fastertools:mcp-transport-http-tools@0.1.0 -o transport.wasm

# Tools with auth
wkg get fastertools:mcp-transport-http-tools-auth@0.1.0 -o transport.wasm
```

### Composition

```bash
wac plug --plug provider.wasm transport.wasm -o server.wasm
```

## Building

```bash
# Tools only
cargo component build --release --no-default-features --features tools

# Tools with auth
cargo component build --release --no-default-features --features "tools auth"
```

## Published Packages

- `fastertools:mcp-transport-http-tools@0.1.0`
- `fastertools:mcp-transport-http-tools-auth@0.1.0`

## Implementation

Written in Rust using:
- `spin-sdk` for HTTP handling
- `rmcp` for JSON-RPC processing
- WASI for runtime compatibility

## Size

~550KB per variant

## License

Apache-2.0