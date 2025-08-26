# python-echo

An MCP server written in Python with echo, text manipulation, and weather tools

## Structure

This is a Spin application that implements the Model Context Protocol (MCP) using WebAssembly components.

- `handler/` - The Python implementation of your MCP handler
- `spin.toml` - Spin application manifest  
- `Makefile` - Build and development commands
- `wasmcp-spin.wasm` - Pre-built gateway component

## Development

### Prerequisites

- Python 3.11+
- Spin CLI
- componentize-py (installed automatically by Makefile)

### Building

```bash
# Build the handler component
make build

# Compose with gateway
make compose
```

### Testing

The handler can be tested locally in Python before building to WASM:

```bash
make test
```

This runs the handler in pure Python mode to verify:
- Tool registration and metadata
- Tool execution with sample inputs
- Resource and prompt definitions

### Running Locally

#### With Spin (recommended)
```bash
# Build and run in one command
make run
```

The MCP server will be available at `http://localhost:3000/mcp`

#### With Wasmtime (standalone WASI runtime)
```bash
# Build, compose, and run with wasmtime
make run-wasmtime
```

The MCP server will be available at `http://localhost:8080`

### Example Usage

```bash
# List available tools
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "id": 1
  }'

# Call the echo tool
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "echo",
      "arguments": {
        "message": "Hello from Python MCP!"
      }
    },
    "id": 2
  }'

# Get weather information (mock data)
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "weather",
      "arguments": {
        "location": "San Francisco"
      }
    },
    "id": 3
  }'

# List available resources
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "resources/list",
    "id": 4
  }'
```

### Convenient Testing Commands

The Makefile includes several test commands for easy development:

```bash
make test-tools      # Test tools/list endpoint
make test-echo       # Test echo tool
make test-weather    # Test weather tool  
make test-reverse    # Test reverse tool
make test-resources  # Test resources/list
make test-prompts    # Test prompts/list
make test-all        # Run all tests
```

## Implementing Your Tools

Edit `handler/src/app.py` to add new tools using the decorator API:

### Adding a New Tool

```python
@handler.tool(description="Your tool description")
def my_new_tool(param1: str, param2: int = 42) -> str:
    """Tool documentation.
    
    Args:
        param1: Description of parameter 1
        param2: Optional parameter with default value
        
    Returns:
        Result description
    """
    # Your tool logic here
    return f"Processed {param1} with {param2}"
```

### Adding a Resource

```python
@handler.resource(
    uri="data://my-resource",
    name="My Resource",
    mime_type="application/json",
    description="Resource description"
)
def get_my_resource() -> dict:
    """Get my resource data."""
    return {"key": "value", "data": [1, 2, 3]}
```

### Adding a Prompt

```python
@handler.prompt(description="Generate a custom prompt")
def my_prompt(context: str = "general") -> list:
    """Generate a custom prompt.
    
    Args:
        context: Context for the prompt
        
    Returns:
        List of prompt messages
    """
    return [
        {"role": "system", "content": f"You are an expert in {context}."},
        {"role": "user", "content": "Provide guidance based on the context."}
    ]
```

## Key Features

### Decorator-Based API
The Python SDK uses decorators for clean, intuitive tool definition:

```python
from wasmcp import Handler

handler = Handler("my-handler")

@handler.tool
def simple_tool(input: str) -> str:
    return f"Processed: {input}"
```

### Automatic Schema Generation
Input/output schemas are automatically generated from function signatures and type hints.

### Built-in Validation  
The SDK automatically validates inputs against generated schemas.

### WebAssembly Compilation
Python code compiles to WebAssembly using componentize-py while preserving the decorator API.

## Configuration

### Spin Configuration

Edit `spin.toml` to configure:
- Application metadata
- HTTP routes and triggers
- Component dependencies
- Build commands and file watching

### Python Dependencies

Edit `handler/requirements.txt` to add Python packages needed by your handler.

### componentize-py Configuration  

The `handler/componentize-py.toml` configures WebAssembly compilation:
- WIT directory location
- Python bindings output location
- Module resolution settings

## Architecture

This example uses the **gateway component pattern**:

1. `wasmcp-spin.wasm` - Pre-built gateway that handles HTTP and MCP protocol
2. `handler/app.wasm` - Your Python handler compiled to WebAssembly
3. Spin composes them together using component dependencies

This architecture provides:
- ✅ Clean separation between protocol handling and business logic
- ✅ Reusable gateway across different language implementations
- ✅ Optimized performance with minimal overhead
- ✅ Easy testing and development of handlers independently