# {{project-name}}

{{project-description}}

## Quick Start

```bash
make setup  # Install dependencies and configure tools
make build  # Build and compose Wasm components
make serve  # Run server on port 8080
```

## Architecture

This implementation uses WIT bindings directly as the SDK, providing transparent access to the MCP protocol. The approach eliminates abstraction layers, making the protocol implementation explicit and debuggable.

Components composed at build time:
- Provider component (this code) - exports MCP capabilities
- HTTP transport v0.4.1 (from registry) - handles JSON-RPC over HTTP
- Optional OAuth 2.0 authorization

## Example Tools

This server implements three demonstration tools:

- **`echo`** - Simple message echo for testing
- **`get_weather`** - Fetch weather for a single location
- **`multi_weather`** - Concurrent weather fetching for multiple cities (demonstrates PollLoop async)

## Development

### Prerequisites

- Python 3.10+
- componentize-py
- wac
- wkg

### Project Structure

```
app.py           # MCP capabilities implementation
wit/             # WIT interface definitions (wasmcp:mcp@0.1.0)
wit_world/       # Generated Python bindings (auto-generated)
requirements.txt # Python dependencies
Makefile        # Build automation
```

### Implementing Tools

Tools are defined in the `TOOLS` dictionary and handled in `handle_call_tool`:

```python
class WeatherMCPCapabilities(ToolsCapabilities, CoreCapabilities):
    TOOLS = {
        "get_weather": {
            "description": "Get current weather for a location",
            "schema": {
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "City name to get weather for"
                    }
                },
                "required": ["location"]
            }
        }
    }
    
    def handle_call_tool(self, request: CallToolRequest) -> ToolResult:
        if request.name == "get_weather":
            arguments = json.loads(request.arguments or "{}")
            location = arguments.get("location", "")
            
            # Fetch weather data
            result = self._get_weather_for_city(location)
            return self._text_result(result)
```

## Concurrency

Python's Wasm environment uses PollLoop for async operations. Example from the multi-weather implementation:

```python
def _handle_multi_weather(self, arguments: dict) -> str:
    cities = arguments.get("cities", [])
    
    # PollLoop enables async operations in Wasm
    loop = PollLoop()
    asyncio.set_event_loop(loop)
    try:
        return loop.run_until_complete(self._fetch_weather_concurrent(cities))
    finally:
        loop.close()

async def _fetch_weather_concurrent(self, cities: List[str]) -> str:
    # Concurrent HTTP requests
    tasks = [self._fetch_weather_async(city) for city in cities]
    results = await asyncio.gather(*tasks, return_exceptions=True)
    return self._format_results(results)
```

## Testing

```bash
make test-all    # Run all tests
make test-echo   # Test echo tool
```

## Authorization

OAuth 2.0 authorization is optional and configured in the `get_auth_config` method:

```python
def get_auth_config(self) -> Optional[ProviderAuthConfig]:
    # Return None to disable authorization
    return None
    
    # Or enable OAuth 2.0 protection:
    # return ProviderAuthConfig(
    #     expected_issuer="https://your-domain.authkit.app",
    #     expected_audiences=["client_id"],
    #     jwks_uri="https://your-domain.authkit.app/oauth2/jwks",
    #     policy=None,      # Optional Rego policy string
    #     policy_data=None  # Optional policy data JSON
    # )
```

The transport component handles:
- JWT validation
- JWKS fetching and caching
- OAuth discovery endpoints
- Rego policy evaluation (if configured)

## Deployment

```bash
# Local development with Wasmtime
wasmtime serve -Scli mcp-http-server.wasm

# Spin framework
spin up --from mcp-http-server.wasm

# Deploy to Fermyon Cloud
spin cloud deploy
```

## License

Apache-2.0