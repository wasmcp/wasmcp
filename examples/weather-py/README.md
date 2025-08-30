# weather-py

An MCP server written in Python

## Quick Start

```bash
# Setup (first time only)
make setup

# Build the component
make build

# Run the server
make serve
```

## Testing

Test the MCP server with curl:

```bash
# List available tools
make test-tools

# Test echo tool
make test-echo

# Test weather tool
make test-weather

# Test multi-weather tool
make test-multi
```

## Development

### Project Structure

```
.
├── app.py                 # Your MCP handler implementation
├── composed.wasm         # Final composed component (handler + server)
├── wit/                  # WebAssembly Interface Types
│   ├── world.wit        # World definition
│   └── deps/            # MCP interface dependencies
├── wit_world/           # Generated Python bindings
├── venv/                # Python virtual environment
└── requirements.txt     # Python dependencies
```

### Adding New Tools

1. Add tool definition in `handle_list_tools()`:
```python
tools.Tool(
    base=mcp_types.BaseMetadata(name="my_tool", title="My Tool"),
    description="What this tool does",
    input_schema=json.dumps({
        "type": "object",
        "properties": {
            "param": {"type": "string", "description": "Parameter description"}
        },
        "required": ["param"]
    }),
    output_schema=None,
    annotations=None,
    meta=None
)
```

2. Handle the tool in `handle_call_tool()`:
```python
elif request.name == "my_tool":
    return self._handle_my_tool(args)
```

3. Implement the handler:
```python
def _handle_my_tool(self, args: dict) -> tools.ToolResult:
    param = args.get("param")
    # Your implementation here
    return self._success(f"Result: {param}")
```

### HTTP Requests

This template uses componentize-py's built-in poll_loop for async HTTP:

```python
async def _fetch_json(self, url: str) -> dict:
    request = OutgoingRequest(Fields.from_list([]))
    request.set_scheme(Scheme_Https())
    request.set_authority(parsed.netloc)
    request.set_path_with_query(path_with_query)
    request.set_method(Method_Get())
    
    response = await poll_loop.send(request)
    # ... handle response
```

### Debugging

```bash
# Test Python code locally (without WASM)
make test-local

# Regenerate bindings if WIT files change
make bindgen

# Clean all build artifacts
make clean
```

## Deployment

The `composed.wasm` file is a standalone WebAssembly component that can run on:

- **Wasmtime**: `wasmtime serve -Scli composed.wasm`
- **Spin**: `spin up`
- **Any WASI-compliant runtime**

## Requirements

- Python 3.10+
- componentize-py
- wkg (WebAssembly package manager)
- wac (WebAssembly component tools)
- wasmtime or Spin runtime

## License

Apache-2.0