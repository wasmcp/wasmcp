# String Tools (Python)

A Python example demonstrating MCP tools implementation using `componentize-py`. Provides string manipulation operations with Python's native slicing semantics.

## Overview

The strings-py component shows how to:

- **Build Python components**: Use `componentize-py` to create WebAssembly components
- **Implement tools capability**: Export `wasmcp:mcp-v20250618/tools` interface
- **Parse JSON arguments**: Handle tool inputs in Python
- **Send notifications**: Log messages during tool execution
- **Python slicing**: Use Python's familiar slicing syntax

This example demonstrates that wasmcp supports multiple programming languages for component development.

## Tools Provided

| Tool | Description | Arguments |
|------|-------------|-----------|
| `reverse` | Reverse a string | `text: string` |
| `slice` | Extract substring using Python slicing | `text: string, start: int, end?: int` |

## Quick Start

```bash
# Create Python virtual environment
python3 -m venv venv
source venv/bin/activate

# Install dependencies
pip install componentize-py

# Build the component
make build
# Creates: strings.wasm

# Compose into MCP server
make compose
# Creates: mcp-server.wasm

# Run with Spin
spin up

# In another terminal, test the tools
wasmcp mcp call-tool reverse '{"text":"Hello World"}'
# Result: "dlroW olleH"

wasmcp mcp call-tool slice '{"text":"Python","start":0,"end":2}'
# Result: "Py"
```

## Building

### Prerequisites

```bash
# Python 3.10 or higher
python3 --version

# Create virtual environment
python3 -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install componentize-py
pip install componentize-py
```

### Build Process

```bash
# Build the WebAssembly component
make build
# This runs two steps:
# 1. componentize-py --wit-path wit --world strings bindings .
# 2. componentize-py --wit-path wit --world strings componentize app -o strings.wasm

# Compose into complete MCP server
make compose

# Clean build artifacts
make clean
```

The build process:
1. Generates Python bindings from WIT files (`wit_world/` module)
2. Reads `app.py` (your Python code)
3. Reads `wit/world.wit` (component interface definition)
4. Creates `strings.wasm` (WebAssembly component)

## Implementation Guide

### 1. Define Your Component World

```wit
// wit/world.wit
package wasmcp:strings@0.1.0;

world strings {
    // Import server-io for notifications
    import wasmcp:mcp-v20250618/server-io@0.1.7;

    // Export tools capability
    export wasmcp:mcp-v20250618/tools@0.1.7;
}
```

**Note**: MessageContext (called `RequestCtx` in Python) is automatically provided through the tools interface - no explicit import needed.

### 2. Implement the Tools Interface

```python
"""String Tools Capability Provider"""

import json
from typing import Optional
from wit_world import exports
from wit_world.imports import mcp, server_handler, server_io


class StringsTools(exports.Tools):
    def list_tools(
        self,
        ctx: server_handler.RequestCtx,
        request: mcp.ListToolsRequest,
    ) -> mcp.ListToolsResult:
        return mcp.ListToolsResult(
            tools=[
                mcp.Tool(
                    name="reverse",
                    input_schema=json.dumps({
                        "type": "object",
                        "properties": {
                            "text": {"type": "string", "description": "Text to reverse"}
                        },
                        "required": ["text"]
                    }),
                    options=None,
                ),
                mcp.Tool(
                    name="slice",
                    input_schema=json.dumps({
                        "type": "object",
                        "properties": {
                            "text": {"type": "string", "description": "Text to slice"},
                            "start": {"type": "integer", "description": "Start index"},
                            "end": {"type": "integer", "description": "End index (optional)"}
                        },
                        "required": ["text", "start"]
                    }),
                    options=mcp.ToolOptions(
                        description="Extract substring by start/end indices",
                        title="Slice",
                    ),
                ),
            ],
            meta=None,
            next_cursor=None,
        )

    def call_tool(
        self,
        ctx: server_handler.RequestCtx,
        request: mcp.CallToolRequest,
    ) -> Optional[mcp.CallToolResult]:
        if not request.arguments:
            return error_result("Missing tool arguments")

        try:
            args = json.loads(request.arguments)
        except json.JSONDecodeError as e:
            return error_result(f"Invalid JSON arguments: {e}")

        if request.name == "reverse":
            return reverse_string(args.get("text"))
        elif request.name == "slice":
            return slice_string(
                args.get("text"),
                args.get("start"),
                args.get("end")
            )
        else:
            return None  # Tool not handled


# Export the implementation
Tools = StringsTools
```

**Key points**:
- Import from `wit_world` (generated by componentize-py)
- Inherit from `exports.Tools`
- Return `None` if tool not handled (allows composition)
- Export your class as `Tools`

### 3. Argument Parsing

```python
def call_tool(
    self,
    ctx: server_handler.RequestCtx,
    request: mcp.CallToolRequest,
) -> Optional[mcp.CallToolResult]:
    # Parse JSON arguments
    try:
        args = json.loads(request.arguments)
    except json.JSONDecodeError as e:
        return error_result(f"Invalid JSON arguments: {e}")

    # Extract parameters
    text = args.get("text")
    start = args.get("start")
    end = args.get("end")  # Optional parameter

    # Validate types
    if not isinstance(text, str):
        return error_result("Missing or invalid parameter 'text'")
    if not isinstance(start, int):
        return error_result("Missing or invalid parameter 'start'")

    # Use the parameters
    return slice_string(text, start, end)
```

### 4. Result Construction

```python
def success_result(text: str) -> mcp.CallToolResult:
    return mcp.CallToolResult(
        content=[mcp.ContentBlock_Text(mcp.TextContent(
            text=mcp.TextData_Text(text),
            options=None,
        ))],
        is_error=None,  # or False
        meta=None,
        structured_content=None,
    )


def error_result(message: str) -> mcp.CallToolResult:
    return mcp.CallToolResult(
        content=[mcp.ContentBlock_Text(mcp.TextContent(
            text=mcp.TextData_Text(message),
            options=None,
        ))],
        is_error=True,  # Mark as error
        meta=None,
        structured_content=None,
    )
```

### 5. Sending Notifications

Send log notifications during tool execution:

```python
from wit_world.imports import server_io, mcp

def call_tool(
    self,
    ctx: server_handler.RequestCtx,
    request: mcp.CallToolRequest,
) -> Optional[mcp.CallToolResult]:
    # Helper function to log messages
    log = lambda message: server_io.notify(
        ctx.messages,
        mcp.ServerNotification_Log(value=mcp.LoggingMessageNotification(
            data=message,
            level=mcp.LogLevel.INFO,
            logger="python-tools"
        ))
    )

    # Use logging in your tool
    if request.name == "slice":
        text = args.get("text")
        start = args.get("start")
        end = args.get("end")

        log(f"slicing text='{text}' from {start} to {end}")

        return slice_string(text, start, end)
```

**Notes**:
- Use `ctx.messages` for the message stream
- `ServerNotification_Log` creates log notifications
- Failures are silently ignored (best-effort)

### 6. Python String Operations

Leverage Python's built-in string methods:

```python
def reverse_string(text: str) -> mcp.CallToolResult:
    """Reverse a string using Python slicing"""
    if not isinstance(text, str):
        return error_result("Missing or invalid parameter 'text'")

    # Python's slice notation: [::-1] reverses
    return success_result(text[::-1])


def slice_string(
    text: str,
    start: int,
    end: Optional[int]
) -> mcp.CallToolResult:
    """Extract substring using Python slicing semantics"""
    if not isinstance(text, str):
        return error_result("Missing or invalid parameter 'text'")
    if not isinstance(start, int):
        return error_result("Missing or invalid parameter 'start'")

    # Python slicing: text[start:end]
    result = text[start:end] if end is not None else text[start:]
    return success_result(result)
```

**Python slicing features**:
- `text[start:]` - From start to end
- `text[start:end]` - From start to end (exclusive)
- `text[::-1]` - Reverse
- Negative indices work: `text[-3:]` gets last 3 characters

## Testing

### With wasmcp CLI

```bash
# Start server
spin up

# Initialize session
wasmcp mcp initialize

# Reverse a string
wasmcp mcp call-tool reverse '{"text":"Hello World"}'
# Result: "dlroW olleH"

# Slice operations
wasmcp mcp call-tool slice '{"text":"Python","start":0,"end":2}'
# Result: "Py"

wasmcp mcp call-tool slice '{"text":"Python","start":2}'
# Result: "thon"

# Negative indices work!
wasmcp mcp call-tool slice '{"text":"Python","start":-2}'
# Result: "on"
```

### With curl

```bash
# Initialize session
SESSION_ID=$(curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -D - \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' \
  | grep -i "mcp-session-id" | cut -d' ' -f2 | tr -d '\r')

# Call reverse
curl -X POST http://localhost:3000/mcp \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"reverse","arguments":"{\"text\":\"OpenAI\"}"}}'
```

## Integration with Claude Code

Add to Claude Code MCP server configuration:

```json
{
  "mcpServers": {
    "strings": {
      "command": "spin",
      "args": [
        "up",
        "--listen",
        "127.0.0.1:3000",
        "--from",
        "/path/to/strings-py"
      ]
    }
  }
}
```

Claude Code can then use the string tools:

```
User: Reverse the string "Hello World"
Claude: I'll use the reverse tool.
[calls reverse tool with {"text": "Hello World"}]
Claude: The reversed string is "dlroW olleH"
```

## Python Best Practices

### 1. Type Hints

Use type hints for better code clarity:

```python
from typing import Optional

def slice_string(
    text: str,
    start: int,
    end: Optional[int]
) -> mcp.CallToolResult:
    """Extract substring with type hints"""
    # Implementation...
```

### 2. Input Validation

Validate inputs thoroughly:

```python
def validate_text_arg(args: dict, key: str) -> tuple[bool, Optional[str]]:
    """Validate string argument"""
    value = args.get(key)
    if value is None:
        return False, f"Missing parameter '{key}'"
    if not isinstance(value, str):
        return False, f"Parameter '{key}' must be a string"
    return True, None

# Usage
valid, error = validate_text_arg(args, "text")
if not valid:
    return error_result(error)
```

### 3. Error Handling

```python
def call_tool(
    self,
    ctx: server_handler.RequestCtx,
    request: mcp.CallToolRequest,
) -> Optional[mcp.CallToolResult]:
    try:
        args = json.loads(request.arguments)
    except json.JSONDecodeError as e:
        return error_result(f"Invalid JSON: {e}")
    except Exception as e:
        return error_result(f"Unexpected error: {e}")

    # Tool logic...
```

### 4. Docstrings

Document your tools clearly:

```python
def reverse_string(text: str) -> mcp.CallToolResult:
    """Reverse a string using Python slicing.

    Args:
        text: The string to reverse

    Returns:
        CallToolResult with reversed text or error

    Example:
        reverse_string("hello") -> "olleh"
    """
    if not isinstance(text, str):
        return error_result("Missing or invalid parameter 'text'")

    return success_result(text[::-1])
```

## Advanced Patterns

### Adding Tool Metadata

```python
mcp.Tool(
    name="reverse",
    input_schema=json.dumps({...}),
    options=mcp.ToolOptions(
        title="Reverse String",
        description="Reverse a string using Python slicing",
        meta=json.dumps({
            "component_id": "strings-py",
            "tags": {
                "category": "text",
                "language": "python"
            }
        }),
        annotations=mcp.ToolAnnotations(
            read_only_hint=True,
            idempotent_hint=True,
        ),
    ),
)
```

### Structured Content

Return both text and structured data:

```python
def success_with_metadata(text: str, metadata: dict) -> mcp.CallToolResult:
    return mcp.CallToolResult(
        content=[mcp.ContentBlock_Text(mcp.TextContent(
            text=mcp.TextData_Text(text),
            options=None,
        ))],
        structured_content=json.dumps(metadata),
        is_error=None,
        meta=None,
    )

# Usage
return success_with_metadata(
    result,
    {"length": len(result), "operation": "slice"}
)
```

## Composing with Other Components

```bash
# With calculator (Rust) and strings (Python)
wasmcp compose server \
  calculator.wasm \
  strings.wasm \
  -o mcp-server.wasm

# With middleware
wasmcp compose server \
  counter_middleware.wasm \
  strings.wasm \
  -o mcp-server.wasm
```

All tools from all components are available at the same MCP endpoint.

## Troubleshooting

### Build Errors

**Problem**: `ModuleNotFoundError: No module named 'componentize'`

**Solution**: Install componentize-py in your virtual environment:
```bash
pip install componentize-py
```

**Problem**: `wit-bindgen-py` not found

**Solution**: componentize-py includes wit-bindgen-py, reinstall:
```bash
pip install --force-reinstall componentize-py
```

### Runtime Errors

**Problem**: Tool returns "Missing tool arguments"

**Solution**: Ensure arguments are passed as JSON string:
```bash
# Correct
wasmcp mcp call-tool reverse '{"text":"hello"}'

# Incorrect
wasmcp mcp call-tool reverse text=hello
```

**Problem**: JSON decode error

**Solution**: Check JSON syntax and escape quotes properly

## Performance Notes

- Python components have more overhead than Rust/native components
- Good for text processing, data transformation
- Not ideal for high-performance numeric operations
- Consider Rust for performance-critical tools

## Files

```
strings-py/
├── app.py               # Python tool implementation
├── Makefile             # Build targets
├── README.md            # This file
├── requirements.txt     # Python dependencies
├── spin.toml            # Spin runtime configuration
├── strings.wasm         # Built component (created by build)
├── wit/
│   ├── deps/            # WIT dependencies
│   ├── deps.lock
│   ├── deps.toml
│   └── world.wit       # Component world definition
└── wit_world/           # Generated bindings (created by build)
```

## Related Examples

- **calculator-rs** - Basic tools in Rust
- **weather-ts** - Tools in TypeScript with HTTP
- **todo-list-auth** - Tools with authorization
- **counter-middleware** - Middleware pattern

## Related Documentation

- [ComponentizePy](https://github.com/bytecodealliance/componentize-py)
- [Python in WebAssembly](https://docs.python.org/3/using/wasm.html)
- [WebAssembly Component Model](https://component-model.bytecodealliance.org/)
