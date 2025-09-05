# {{project-name}}

{{project-description}}

## Quick Start

```bash
make setup  # Install dependencies
make build  # Build server (no auth)
make serve  # Run on port 8080
```

Authentication is configured in code, not at build time.
See the OAuth Authentication section below.

## Architecture

This template uses a transparent implementation that works directly with WIT bindings,
prioritizing clarity and understanding over abstraction.

WebAssembly components composed at build time:
- Provider component (this code)
- HTTP transport (from registry)
- Authorization (optional)

## Development

### Prerequisites

- Python 3.10+
- componentize-py
- wasm-tools
- wac
- wkg

### Project Structure

```
app.py       # Direct MCP implementation using WIT bindings
wit/         # Interface definitions
wit_world/   # Generated Python bindings (created by build)
```

### Adding Tools

Add tools by updating the `TOOLS` dictionary and implementing handler methods:

```python
class WeatherMCPCapabilities(ToolsCapabilities, CoreCapabilities):
    TOOLS = {
        "my_tool": {
            "description": "Tool description",
            "schema": {
                "type": "object",
                "properties": {
                    "param": {"type": "string", "description": "Parameter"}
                },
                "required": ["param"]
            }
        }
    }
    
    def handle_call_tool(self, request: tool_types.CallToolRequest) -> tool_types.ToolResult:
        # Route to your tool implementation
        if request.name == "my_tool":
            arguments = json.loads(request.arguments)
            result = f"Result: {arguments['param']}"
            return self._text_result(result)
```

## Concurrency

Use PollLoop and asyncio for concurrent operations:

```python
def _execute_multi_fetch_sync(self, arguments: dict) -> str:
    urls = arguments.get("urls", [])
    
    # Run async code in PollLoop
    loop = PollLoop()
    asyncio.set_event_loop(loop)
    try:
        return loop.run_until_complete(self._fetch_multi(urls))
    finally:
        loop.close()

async def _fetch_multi(self, urls: List[str]) -> str:
    tasks = [self._fetch_json(url) for url in urls]
    results = await asyncio.gather(*tasks, return_exceptions=True)
    return json.dumps(results)
```

## Testing

```bash
make test-all    # Run all tests
make test-echo   # Test echo tool
```

## OAuth Authentication

Optional OAuth 2.0/JWT support. Configure in `app.py`:

```python
def get_auth_config(self) -> Optional:
    from wit_world.imports.authorization_types import ProviderAuthConfig
    return ProviderAuthConfig(
        expected_issuer="https://xxxxx.authkit.app",
        expected_audiences=["client_xxxxx"],
        jwks_uri="https://xxxxx.authkit.app/oauth2/jwks",
        policy=None,  # Optional: Add Rego policy
        policy_data=None,  # Optional: Add policy data
    )
```

Features:
- JWT validation with JWKS
- OAuth discovery endpoints
- OPA/Rego policies
- Works with AuthKit, Auth0, etc.

## Runtime Options

```bash
# Wasmtime
wasmtime serve -Scli mcp-http-server.wasm

# Spin (no auth only)
spin up
```

## License

Apache-2.0