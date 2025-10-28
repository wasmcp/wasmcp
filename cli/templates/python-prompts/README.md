# {{project_name}}

MCP prompts capability component in Python.

## Prerequisites

- Python 3.10 or later
- `componentize-py` (installed automatically by Makefile)

## Build

```bash
make  # Output: {{project_name}}.wasm
```

## Compose

```bash
wasmcp compose server {{project_name}}.wasm -o server.wasm
```

The CLI automatically detects this is a prompts-capability component and wraps it with prompts-middleware.

## Run

```bash
# HTTP
wasmtime serve -Scli server.wasm

# Stdio
wasmcp compose server {{project_name}}.wasm -t stdio -o server.wasm
wasmtime run server.wasm
```

## Implementation

This component uses the **capability pattern**, implementing two methods from the `prompts-capability` interface:

- `list_prompts()` - Returns all prompts this component provides
- `get_prompt()` - Returns prompt content by name, or `None` if not handled

See `app.py` for example prompts demonstrating:
- Prompt definitions with names and arguments
- Dynamic prompt generation based on arguments
- No protocol handling or delegation code

The prompts-middleware automatically handles:
- MCP protocol translation
- Merging prompts from multiple components
- Request delegation to downstream components
- Error handling and response formatting

## Adding Prompts

To add new prompts:

1. Add a `Prompt` entry to the list in `list_prompts()`:

```python
mcp.Prompt(
    name="my-prompt",
    options=mcp.PromptOptions(
        meta=None,
        arguments=[
            mcp.PromptArgument(
                name="arg1",
                description="First argument",
                required=True,
                title="Argument 1",
            ),
        ],
        description="Description of my prompt",
        title="My Prompt",
    ),
)
```

2. Add a handler in `get_prompt()`:

```python
elif request.name == "my-prompt":
    import json
    args = json.loads(request.arguments) if request.arguments else {}
    arg1 = args.get("arg1", "default")

    return mcp.GetPromptResult(
        meta=None,
        description="My prompt description",
        messages=[
            mcp.PromptMessage(
                role=mcp.Role.USER,
                content=mcp.ContentBlock(
                    text=mcp.TextContent(
                        text=mcp.TextData(
                            text=f"Your prompt text using {arg1}",
                            text_stream=None,
                        ),
                        options=None,
                    ),
                    image=None,
                    embedded_resource=None,
                    resource=None,
                ),
            ),
        ],
    )
```

3. That's it! No need to handle merging, delegation, or protocol details - the middleware does that for you.

## Clean

```bash
make clean  # Remove venv and build artifacts
```
