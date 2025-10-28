# {{project_name}}

MCP resources capability component in Rust.

## Build

```bash
make setup  # Install wasm32-wasip2 target
make build  # Output: target/wasm32-wasip2/release/{{package_name}}.wasm
```

## Compose

```bash
wasmcp compose server target/wasm32-wasip2/release/{{package_name}}.wasm -o server.wasm
```

The CLI automatically detects this is a resources-capability component and wraps it with resources-middleware.

## Run

```bash
# HTTP
wasmtime serve -Scli server.wasm

# Stdio
wasmcp compose server target/wasm32-wasip2/release/{{package_name}}.wasm -t stdio -o server.wasm
wasmtime run server.wasm
```

## Implementation

This component uses the **capability pattern**, implementing three methods from the `resources-capability` interface:

- `list_resources()` - Returns all resources this component provides
- `read_resource()` - Returns resource content by URI, or `None` if not handled
- `list_resource_templates()` - Returns URI templates (empty for static resources)

See `src/lib.rs` for a simple text resources implementation demonstrating:
- Resource definitions with URIs and metadata
- Static content serving
- No protocol handling or delegation code

The resources-middleware automatically handles:
- MCP protocol translation
- Merging resources from multiple components
- Request delegation to downstream components
- Error handling and response formatting

## Adding Resources

To add new resources:

1. Add a `Resource` entry to the vec in `list_resources()`
2. Add a URI match arm in `read_resource()`
3. Return the resource content

No need to handle merging, delegation, or protocol details - the middleware does that for you!
