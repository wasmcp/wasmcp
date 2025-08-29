# Python MCP Weather Handler

A Python implementation of the MCP weather handler, demonstrating how to build MCP tools with clean, Pythonic APIs.

## Features

- **Pythonic API**: Decorators, type hints, and async/await support
- **Clean Abstractions**: Hide WIT complexity behind simple Python interfaces
- **Async Support**: Native asyncio for concurrent operations
- **Type Inference**: Automatic schema generation from function signatures

## Prerequisites

- Python 3.10 or later
- pip
- componentize-py
- wkg (for fetching the server component)
- wac (for component composition)
- wasmtime (for running the component)

## Quick Start

```bash
# Install dependencies
make install-deps

# Build the component
make build

# Run the server
make run

# In another terminal, test it
make test-weather
```

## Project Structure

```
python/
├── app.py           # Main handler implementation
├── helpers.py       # Helper library for clean Python API
├── wit/            # WIT interface definitions
├── requirements.txt # Python dependencies
└── Makefile        # Build automation
```

## Writing Tools

The Python helpers provide multiple ways to define tools:

### Using Decorators

```python
from helpers import Handler

handler = Handler()

@handler.tool(
    name="my_tool",
    description="Does something useful",
    schema={
        "type": "object",
        "properties": {
            "input": {"type": "string"}
        },
        "required": ["input"]
    }
)
async def my_tool(input: str) -> str:
    return f"Processed: {input}"
```

### Using Classes

```python
from helpers import Tool

class MyTool(Tool):
    @property
    def name(self) -> str:
        return "my_tool"
    
    @property
    def description(self) -> str:
        return "Does something useful"
    
    async def execute(self, args: Dict[str, Any]) -> str:
        return f"Processed: {args['input']}"

handler.register(MyTool())
```

### Type Inference

If you don't provide a schema, the helper will try to infer it from type hints:

```python
@handler.tool(
    name="typed_tool",
    description="Uses type hints for schema"
)
async def typed_tool(
    message: str,
    count: int = 1,
    enabled: bool = True
) -> str:
    # Schema is automatically inferred from the signature
    return f"Message: {message}, Count: {count}, Enabled: {enabled}"
```

## Async Support

All tools can be async for non-blocking I/O:

```python
@handler.tool(name="fetch_data", description="Fetches external data")
async def fetch_data(url: str) -> str:
    async with aiohttp.ClientSession() as session:
        async with session.get(url) as response:
            return await response.text()
```

## Testing

Test locally without building to WASM:

```bash
# Run the Python code directly
make test-local
```

Test the full WASM component:

```bash
# Start the server
make run

# Test individual endpoints
make test-init    # Initialize
make test-tools   # List tools
make test-echo    # Echo tool
make test-weather # Weather tool
make test-multi   # Multi-weather tool
```

## Implementation Details

### Helper Library (`helpers.py`)

The helper library provides:
- `Tool` base class for class-based tools
- `@tool` decorator for function-based tools
- `Handler` class for managing tool collections
- Automatic type inference from function signatures
- Async/await support
- Error handling and result formatting

### Main Application (`app.py`)

Implements three example tools:
1. **echo**: Simple synchronous tool
2. **get_weather**: Async HTTP requests to weather API
3. **multi_weather**: Concurrent weather fetches using asyncio

### Build Process

1. `componentize-py` compiles Python to WASM component
2. `wkg` fetches the pre-built MCP server component
3. `wac plug` composes handler with server
4. Result is a single `composed.wasm` ready to run

## Comparison with Other Languages

| Feature | Python | Rust | JavaScript |
|---------|---------|------|------------|
| Async Support | `async`/`await` | `async`/`await` + Spin executor | Promises/`async`/`await` |
| Type Safety | Type hints (runtime) | Compile-time | Runtime |
| Schema Generation | From type hints | Manual JSON | Manual JSON |
| Decorator Support | Native | Proc macros | Function wrappers |
| HTTP Client | urllib (stdlib) | spin_sdk::http | fetch API |

## Advanced Features

### Custom Annotations

Add metadata to tools:

```python
@handler.tool(
    name="dangerous_tool",
    description="Modifies system state",
    read_only_hint=False,
    destructive_hint=True
)
async def dangerous_tool(confirm: bool) -> str:
    if confirm:
        # Do something destructive
        return "Operation completed"
    return "Operation cancelled"
```

### Structured Output

Return structured data instead of just text:

```python
@handler.tool(name="data_tool", description="Returns structured data")
async def data_tool() -> Dict[str, Any]:
    return {
        "content": [{
            "tag": "text",
            "val": {"text": "Main result"}
        }],
        "structuredContent": json.dumps({
            "data": [1, 2, 3],
            "metadata": {"source": "api"}
        })
    }
```

## Troubleshooting

### ImportError for wit_world

If you see import errors, generate the bindings first:

```bash
make build-bindings
```

### Async Runtime Issues

Ensure you're using Python 3.10+ for proper async support.

### Component Build Failures

Check that componentize-py is installed:

```bash
pip install componentize-py
```

## Contributing

The Python implementation follows the same patterns as Rust and JavaScript for consistency:
- Clean helper abstractions hiding WIT complexity
- Idiomatic language features (decorators, async/await)
- Same example tools (echo, weather, multi-weather)
- Compatible with the same server component

## License

Apache-2.0