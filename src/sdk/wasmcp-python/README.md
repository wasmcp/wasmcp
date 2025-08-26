# wasmcp-python

Python SDK for creating WebAssembly MCP (Model Context Protocol) handlers using the Component Model.

## Features

- **Decorator-based API**: Simple decorators for registering tools, resources, and prompts
- **Type-safe**: Full type hints and JSON Schema generation from Python types
- **WebAssembly ready**: Compiles to WebAssembly using componentize-py
- **No dependencies**: Pure Python with direct WIT bindings
- **MCP compliant**: Full support for MCP protocol specification

## Quick Start

```python
from wasmcp import WasmcpHandler

# Create handler
handler = WasmcpHandler("my-handler")

# Register a tool
@handler.tool
def greet(name: str) -> str:
    """Greet someone by name."""
    return f"Hello, {name}!"

# Register a resource
@handler.resource(uri="config://settings")
def get_settings() -> dict:
    return {"version": "1.0.0"}

# Register a prompt
@handler.prompt
def code_review() -> list:
    return [
        {"role": "system", "content": "You are a code reviewer."},
        {"role": "user", "content": "Review this code: {{code}}"}
    ]

# Export for WASM compilation
Handler = handler.build()
```

## Installation

```bash
pip install wasmcp
```

## Development

```bash
pip install -e ".[dev]"
pytest
```