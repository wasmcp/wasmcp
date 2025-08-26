# Python Echo Handler Example

This example demonstrates how to create a simple MCP handler using the wasmcp Python SDK.

## Features

The echo handler provides:

- **Tools**: 
  - `echo`: Echo back a message
  - `reverse`: Reverse text
  - `shout`: Convert text to uppercase (with custom name)

- **Resources**:
  - `config://version`: Handler version information
  - `data://capabilities`: Handler capabilities

- **Prompts**:
  - `greeting_prompt`: Generate greeting prompts with customizable name

## Running the Example

1. Install dependencies:
   ```bash
   pip install -r requirements.txt
   ```

2. Build and run with Spin:
   ```bash
   spin build
   spin up
   ```

3. Test the MCP endpoint:
   ```bash
   curl -X POST http://localhost:3000/mcp \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc": "2.0", "id": 1, "method": "tools/list"}'
   ```

## Code Structure

- `app.py`: Main handler implementation using decorator-based API
- `requirements.txt`: Python dependencies
- `spin.toml`: Spin application configuration

The handler uses the `WasmcpHandler` class with decorators to register MCP components:

```python
from wasmcp import WasmcpHandler

handler = WasmcpHandler("echo-handler")

@handler.tool
def echo(message: str) -> str:
    return f"Echo: {message}"

# Export for WASM compilation
Handler = handler.build()
```