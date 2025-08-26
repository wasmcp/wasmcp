# Go Weather Example

MCP handler demonstrating concurrent HTTP requests with Go and wasip2.

## Tools

- `echo` - Simple echo tool
- `weather` - Get weather for a location  
- `multi_weather` - Concurrent weather for multiple cities

## Build & Run

```bash
make compose
make run-wasmtime
```

## Test

```bash
# Single weather
make test-weather

# Concurrent requests
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"multi_weather","arguments":{"cities":["London","Paris","Tokyo"]}},"id":1}'
```