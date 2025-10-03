# {{ project_name }}

A tools handler component for Model Context Protocol servers, implemented in Rust.

## Overview

This component implements MCP tools handling with an example echo tool. Build upon this foundation by adding your own tools following the same pattern.

## Prerequisites

- Rust 1.75 or later
- [cargo-component](https://github.com/bytecodealliance/cargo-component)
- [wkg](https://github.com/bytecodealliance/wasm-pkg-tools)

## Building

```bash
make
```

This produces a WebAssembly component at `target/wasm32-wasip2/release/{{ package_name }}.wasm`.

## Composition

This component implements the `wasmcp:mcp/incoming-handler` interface and can be composed with other wasmcp components to create a complete MCP server.

Example composition using `wac`:

```bash
wac compose myserver.wac \
  --dep wasmcp:request=wasmcp_request.wasm \
  --dep wasmcp:initialize-writer=wasmcp_initialize-writer.wasm \
  --dep wasmcp:tools-writer=wasmcp_tools-writer.wasm \
  --dep wasmcp:initialize-handler=wasmcp_initialize-handler.wasm \
  --dep wasmcp:http-transport=wasmcp_http-transport.wasm \
  --dep {{ project_name }}=target/wasm32-wasip2/release/{{ package_name }}.wasm \
  -o mcp-server.wasm
```

## Adding Tools

Edit `src/lib.rs`:

1. Add tool definition to `handle_tools_list()`
2. Add tool handler to `handle_tools_call()` match statement
3. Implement the handler function

Example:

```rust
fn handle_my_tool(arguments: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    #[derive(Deserialize)]
    struct MyToolArgs {
        param: String,
    }

    let args: MyToolArgs = serde_json::from_str(arguments.ok_or("Missing arguments")?)?;

    // Tool logic here
    Ok(format!("Result: {}", args.param))
}
```

## Testing

```bash
cargo test
```
