# weather-ts

MCP server implementation in TypeScript demonstrating weather tools with concurrent HTTP capabilities.

## Quick Start

```bash
make setup  # Install dependencies and configure tools
make build  # Build and compose WASM components
make serve  # Run server on port 8080
```

## Architecture

This implementation uses WIT bindings directly as the SDK, providing transparent access to the MCP protocol. The approach eliminates abstraction layers, making the protocol implementation explicit and debuggable.

Components composed at build time:
- Provider component (this code) - exports MCP capabilities
- HTTP transport v0.4.1 (from registry) - handles JSON-RPC over HTTP
- Optional OAuth 2.0 authentication

## Example Tools

This server implements three demonstration tools:

- **`echo`** - Simple message echo for testing
- **`get_weather`** - Fetch weather for a single location
- **`multi_weather`** - Concurrent weather fetching for multiple cities (demonstrates Promise.all)

## Development

### Prerequisites

- Node.js 20+
- jco
- wac
- wkg

### Project Structure

```
src/
  index.ts       # MCP capabilities implementation
  generated/     # Generated TypeScript bindings (auto-generated)
wit/             # WIT interface definitions (fastertools:mcp@0.4.0)
package.json     # Dependencies and scripts
Makefile         # Build automation
```

### Implementing Tools

Tools are handled directly in the `handleCallTool` function:

```typescript
async handleCallTool(request: CallToolRequest): Promise<ToolResult> {
  try {
    switch (request.name) {
      case 'echo':
        return await handleEcho(request.arguments);
      case 'get_weather':
        return await handleGetWeather(request.arguments);
      case 'multi_weather':
        return await handleMultiWeather(request.arguments);
      default:
        return errorResult(`Unknown tool: ${request.name}`);
    }
  } catch (error) {
    return errorResult(
      `Error executing ${request.name}: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

async function handleGetWeather(args?: string): Promise<ToolResult> {
  const parsedArgs: WeatherArgs = args ? JSON.parse(args) : { location: '' };
  const weather = await getWeatherForCity(parsedArgs.location);
  return textResult(weather);
}
```

## Concurrency

TypeScript's WASM environment uses standard Promise APIs for concurrent operations. Example from the multi-weather implementation:

```typescript
async function handleMultiWeather(args?: string): Promise<ToolResult> {
  const parsedArgs: MultiWeatherArgs = args ? JSON.parse(args) : { cities: [] };
  
  // Concurrent HTTP requests using Promise.all
  const results = await Promise.all(
    parsedArgs.cities.map(async (city) => {
      try {
        const weather = await getWeatherForCity(city);
        return `${weather}\n`;
      } catch (error) {
        return `Error fetching weather for ${city}: ${error instanceof Error ? error.message : String(error)}\n`;
      }
    })
  );
  
  // Format results
  let output = '=== Weather Results ===\n\n';
  for (const result of results) {
    output += result + '\n';
  }
  output += '=== All requests completed ===';
  
  return textResult(output);
}
```

## Testing

```bash
make test-all        # Run all tests
make test-echo       # Test echo tool
make test-weather    # Test weather tool
make test-multi      # Test concurrent fetching
```

## Authentication

OAuth 2.0 authentication is optional and configured in the `getAuthConfig` method:

```typescript
getAuthConfig(): ProviderAuthConfig | undefined {
  // Return undefined to disable authentication
  return undefined;
  
  // Or enable OAuth 2.0 protection:
  // return {
  //   expectedIssuer: 'https://your-domain.authkit.app',
  //   expectedAudiences: ['client_id'],
  //   jwksUri: 'https://your-domain.authkit.app/oauth2/jwks',
  //   policy: undefined,      // Optional Rego policy string
  //   policyData: undefined,  // Optional policy data JSON
  // };
}
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