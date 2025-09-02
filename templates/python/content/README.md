# {{project-name}}

{{project-description}}

## Quick Start

```bash
make setup  # Install dependencies
make build  # Build server
make serve  # Run on port 8080
```

## Architecture

WebAssembly components composed at build time:
- Provider component (this code)
- HTTP transport (from registry)

## Development

### Prerequisites

- Python 3.10+
- componentize-py
- wasm-tools
- wac
- wkg

### Project Structure

```
app.py       # Tool implementations
helpers.py   # MCP SDK decorators
wit/         # Interface definitions
```

### Adding Tools

Use the decorator API:

```python
@mcp.tool
def my_tool(param: str) -> str:
    """Tool description."""
    return f"Result: {param}"

# Or with explicit configuration
@mcp.tool(name="custom_name", description="Custom description")
async def async_tool(data: dict) -> str:
    """Process data asynchronously."""
    result = await process_data(data)
    return json.dumps(result)
```

## Concurrency

Use asyncio for concurrent operations:

```python
@mcp.tool
async def multi_fetch(urls: List[str]) -> str:
    tasks = [fetch_url(url) for url in urls]
    results = await asyncio.gather(*tasks)
    return json.dumps(results)
```

## Testing

```bash
make test-all    # Run all tests
make test-echo   # Test echo tool
```

## Runtime Options

```bash
# Wasmtime
wasmtime serve -Scli mcp-http-server.wasm

# Spin
spin up
```

## License

Apache-2.0