# wasmcp-wasi-python

WASI SDK for Python MCP handlers - provides HTTP, Key-Value storage, and Configuration access.

## Overview

This package provides WASI (WebAssembly System Interface) capabilities for Python MCP handlers running in WebAssembly environments like Spin. It complements the main `wasmcp` SDK by providing access to system resources.

## Features

- 🌐 **HTTP Client**: Make outbound HTTP requests
- 💾 **Key-Value Storage**: Persistent storage (Spin-specific)
- ⚙️ **Configuration**: Access runtime configuration values

## Installation

```bash
pip install wasmcp-wasi
```

## Usage

### HTTP Client

Make outbound HTTP requests from your MCP handlers:

```python
from wasmcp_wasi import http

# Simple GET request
response = http.get("https://api.example.com/data")
if response.ok:
    data = response.json()
    print(f"Got data: {data}")

# POST with JSON body
response = http.post(
    "https://api.example.com/users",
    body={"name": "Alice", "email": "alice@example.com"},
    headers={"Authorization": "Bearer token123"}
)

# Using Request object for more control
request = http.Request(
    url="https://api.example.com/resource",
    method=http.HttpMethod.PUT,
    headers={"Content-Type": "application/json"},
    body=json.dumps({"key": "value"})
)
response = http.send(request)
```

### Key-Value Storage

Store and retrieve data persistently (requires Spin with KV support):

```python
from wasmcp_wasi import keyvalue

# Open a store
store = keyvalue.open("my-store")

# Store different types of data
store.set("user:123", {"name": "Bob", "age": 30})  # JSON
store.set("config:theme", "dark")  # String
store.set("data:binary", b"\x00\x01\x02")  # Binary

# Retrieve data
user = store.get_json("user:123")
theme = store.get_str("config:theme")
binary_data = store.get("data:binary")

# Check existence
if store.exists("user:123"):
    print("User exists")

# List keys with prefix
user_keys = store.list_keys("user:")

# Delete keys
store.delete("user:123")

# Bulk operations
store.set_many({
    "setting:1": "value1",
    "setting:2": "value2"
})
```

### Configuration

Access runtime configuration values:

```python
from wasmcp_wasi import config

# Get configuration value
db_host = config.get("DATABASE_HOST")
if db_host:
    print(f"Database host: {db_host}")

# Get with default
port = config.get_with_default("PORT", "3000")

# Require a configuration value (raises if not found)
api_key = config.require("API_KEY")
```

## Integration with wasmcp

Use WASI capabilities in your MCP handlers:

```python
from wasmcp import WasmcpHandler
from wasmcp_wasi import http, keyvalue, config

handler = WasmcpHandler()

@handler.tool
def fetch_weather(city: str) -> dict:
    """Fetch weather data for a city."""
    # Use configuration for API key
    api_key = config.require("WEATHER_API_KEY")
    
    # Make HTTP request
    response = http.get(
        f"https://api.weather.com/v1/weather?city={city}",
        headers={"X-API-Key": api_key}
    )
    
    if response.ok:
        # Cache result in KV store
        store = keyvalue.open()
        store.set(f"weather:{city}", response.json())
        return response.json()
    else:
        # Try to get from cache
        store = keyvalue.open()
        cached = store.get_json(f"weather:{city}")
        if cached:
            return cached
        raise Exception(f"Failed to fetch weather: {response.status}")

@handler.resource(uri="cache://stats")
def get_cache_stats() -> dict:
    """Get cache statistics."""
    store = keyvalue.open()
    keys = store.list_keys("weather:")
    return {
        "cached_cities": len(keys),
        "cities": [k.replace("weather:", "") for k in keys]
    }
```

## Environment Requirements

### Spin Configuration

Add to your `spin.toml`:

```toml
[component.my-handler]
# Enable key-value storage
key_value_stores = ["default"]

# Allow outbound HTTP
allowed_outbound_hosts = [
    "https://api.example.com",
    "https://*.weather.com"
]

# Configuration variables
[component.my-handler.variables]
api_key = { required = true }
database_host = { default = "localhost" }
```

## Limitations

- **Key-Value Storage**: Only available in Spin environments with KV support
- **HTTP Client**: Requires allowed hosts configuration in Spin
- **Configuration**: Limited to string values
- **Async**: Currently uses synchronous APIs (async support planned)

## Error Handling

All WASI operations can fail due to runtime restrictions:

```python
from wasmcp_wasi import keyvalue

try:
    store = keyvalue.open()
    value = store.get("key")
except RuntimeError as e:
    print(f"KV not available: {e}")
except Exception as e:
    print(f"Operation failed: {e}")
```

## Development

### Testing

Since WASI capabilities require a WebAssembly runtime, tests need to run in Spin:

```bash
# Build your component
spin build

# Run with test configuration
spin up --env TEST_MODE=true
```

## License

MIT License - see [LICENSE](LICENSE) file for details.