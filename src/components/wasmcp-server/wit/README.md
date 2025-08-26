# wasmcp HTTP Component WIT

This directory contains the WebAssembly Interface Types (WIT) definitions for the wasmcp HTTP gateway component.

## Structure

- `world.wit` - Defines the `mcp-http-api` world that imports the MCP handler interface

## Dependencies

The MCP handler interface is imported from the root `/wit/mcp.wit` package via the Cargo.toml configuration:

```toml
[package.metadata.component.target.dependencies]
"wasmcp:mcp" = { path = "../../wit" }
```

This eliminates duplication and ensures consistency across the project.

## Building

This component uses `cargo-component` for building. The bindings are generated automatically during the build process.

```bash
cargo component build
```