# {{project_name}}

MCP tools capability component in Python.

## Build

```bash
make setup  # Create venv and install componentize-py
make build  # Output: target/{{project_name}}.wasm
```

## Compose

```bash
wasmcp compose target/{{project_name}}.wasm -o server.wasm
```

The CLI automatically detects this is a tools-capability component and wraps it with tools-middleware.

## Run

```bash
# HTTP
wasmtime serve -Scommon server.wasm

# Stdio
wasmcp compose target/{{project_name}}.wasm -t stdio -o server.wasm
wasmtime run server.wasm
```

## Implementation

This component uses the **capability pattern**, implementing just two methods from the `tools-capability` interface:

- `list_tools()` - Returns all tools this component provides
- `call_tool()` - Executes a tool, returning a result if handled, `None` otherwise

See `app.py` for a string manipulation implementation demonstrating:
- Tool definitions with JSON schemas
- Simple tool execution logic
- No protocol handling or delegation code

The tools-middleware automatically handles:
- MCP protocol translation
- Merging tools from multiple components
- Request delegation to downstream components
- Error handling and response formatting

## Adding Tools

To add new tools:

1. Create a tool definition method (like `_create_reverse_tool()`)
2. Add it to the list in `list_tools()`
3. Add a handler in the `call_tool()` conditional
4. Implement the execution logic

No need to handle merging, delegation, or protocol details - the middleware does that for you!
