# weather-py

An MCP server written in Python

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
- HTTP transport v0.2.0 (from registry) - handles JSON-RPC over HTTP
- Optional OAuth 2.0 authorization

## Example Tools

This server implements three demonstration tools:

- **`echo`** - Simple message echo for testing
- **`get_weather`** - Fetch weather for a single location
- **`multi_weather`** - Concurrent weather fetching for multiple cities (demonstrates PollLoop async)

## Development

### Prerequisites

- Python 3.10+
- componentize-py 0.13.2+
- wac (WebAssembly Composition)
- wkg (WebAssembly package manager)

### Project Structure

```
app.py                    # Entry point - exports capability implementations
capabilities/             # MCP capability implementations
├── __init__.py
├── authorization.py      # OAuth 2.0 configuration
├── lifecycle.py          # Server initialization and lifecycle
└── tools.py              # Tool implementations
wit/                      # WIT interface definitions (wasmcp:mcp@0.2.0)
wit_world/                # Generated Python bindings (auto-generated)
requirements.txt          # Python dependencies
Makefile                  # Build automation
setup.sh                  # Initial setup script
```

### Implementing Tools

Tools are implemented in `capabilities/tools.py`:

```python
class Tools:
    def list_tools(self, request: ListToolsRequest) -> ListToolsResult:
        tools = [
            Tool(
                name="get_weather",
                title="get_weather",
                description="Get current weather for a location",
                input_schema=json.dumps({
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "City name to get weather for"
                        }
                    },
                    "required": ["location"]
                })
            )
        ]
        return ListToolsResult(tools=tools, next_cursor=None)
    
    def call_tool(self, request: CallToolRequest, context: Optional[AuthContext]) -> CallToolResult:
        if request.name == "get_weather":
            args = json.loads(request.arguments or "{}")
            result_text = execute_weather_sync(args)
            return text_result(result_text)
```

## Concurrency

Python's Wasm environment uses PollLoop for async operations. Example from the multi-weather implementation:

```python
def execute_multi_weather_sync(args: Dict[str, Any]) -> str:
    cities = args.get("cities", [])
    
    # PollLoop enables async operations in Wasm
    loop = PollLoop()
    asyncio.set_event_loop(loop)
    try:
        return loop.run_until_complete(fetch_multi_weather(cities))
    finally:
        loop.close()

async def fetch_multi_weather(cities: List[str]) -> str:
    # Concurrent HTTP requests using WASI HTTP
    tasks = [fetch_weather(city) for city in cities]
    results = await asyncio.gather(*tasks, return_exceptions=True)
    
    output = "=== Weather Results ===\n\n"
    for city, result in zip(cities, results):
        if isinstance(result, Exception):
            output += f"Error fetching weather for {city}: {result}\n\n"
        else:
            output += format_weather(result) + "\n\n"
    return output
```

## Testing

```bash
make test-all        # Run all tests
make test-init       # Test initialization
make test-tools      # Test tools/list
make test-echo       # Test echo tool
make test-weather    # Test get_weather tool
make test-multi      # Test multi_weather tool
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
wasmtime serve -Scli build/mcp-http-server.wasm

# Spin framework
spin up --from build/mcp-http-server.wasm

# Deploy to Fermyon Cloud
spin cloud deploy
```

## License

Apache-2.0