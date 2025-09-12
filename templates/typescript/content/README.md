# {{project-name | kebab_case}}

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
- HTTP transport v0.2.0 (from registry) - handles JSON-RPC over HTTP
- Optional OAuth 2.0 authorization

## Example Tools

This server implements three demonstration tools:

- **`echo`** - Simple message echo for testing
- **`get_weather`** - Fetch weather for a single location
- **`multi_weather`** - Concurrent weather fetching for multiple cities (demonstrates Promise.all)

## Development

### Prerequisites

- Node.js 20+
- jco 1.14.0+ (JavaScript Component Objects)
- wac (WebAssembly Composition)
- wkg (WebAssembly package manager)

### Project Structure

```
src/
├── index.ts            # Entry point - exports capability implementations
├── capabilities/       # MCP capability implementations
│   ├── authorization.ts # OAuth 2.0 configuration
│   ├── lifecycle.ts     # Server initialization and lifecycle
│   └── tools.ts         # Tool implementations with Zod schemas
└── generated/          # Generated TypeScript bindings (auto-generated)
wit/                    # WIT interface definitions (wasmcp:mcp@0.2.0)
package.json            # Dependencies and scripts
tsconfig.json           # TypeScript configuration
webpack.config.js       # Bundling configuration
Makefile                # Build automation
setup.sh                # Initial setup script
```

### Implementing Tools

Tools are implemented in `src/capabilities/tools.ts` using Zod for schema validation:

```typescript
import { z } from 'zod';

const WeatherSchema = z.object({
  location: z.string().describe('City name to get weather for'),
});

export function listTools(_request: ListToolsRequest): ListToolsResult {
  return {
    tools: [
      {
        name: 'get_weather',
        title: 'get_weather',
        description: 'Get current weather for a location',
        icons: undefined,
        inputSchema: z.toJSONSchema(WeatherSchema), // Zod v4 built-in
      },
    ],
    nextCursor: undefined,
  };
}

export async function callTool(
  request: CallToolRequest,
  _context: AuthContext | undefined
): Promise<CallToolResult> {
  const args = request.arguments ? JSON.parse(request.arguments) : {};
  
  switch (request.name) {
    case 'get_weather': {
      const validated = WeatherSchema.parse(args);
      return await handleGetWeather(validated);
    }
    default:
      throw new Error(`Unknown tool: ${request.name}`);
  }
}
```

## Concurrency

TypeScript's Wasm environment uses standard Promise APIs for concurrent operations. Example from the multi-weather implementation:

```typescript
async function handleMultiWeather(args: MultiWeatherArgs): Promise<CallToolResult> {
  // Concurrent HTTP requests using Promise.all
  // jco transparently handles the async-to-sync bridge
  const results = await Promise.all(
    args.cities.map(async (city) => {
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
    output += result;
  }
  output += '\n=== All requests completed ===';
  
  return textResult(output);
}
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

OAuth 2.0 authorization is optional and configured in the `getAuthConfig` method:

```typescript
export function getAuthConfig(): ProviderAuthConfig | undefined {
  // Return undefined to disable authorization
  return undefined;
  
  // Or enable OAuth 2.0 protection:
  // return {
  //   expectedIssuer: 'https://your-domain.authkit.app',
  //   expectedAudiences: ['client_id'],
  //   jwksUri: 'https://your-domain.authkit.app/oauth2/jwks',
  //   passJwt: false,
  //   expectedSubject: undefined,
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
# Local development with Wasmtime (needs 8MB memory for concurrent requests)
wasmtime serve -Scli -Wmemory-max-pages=128 mcp-http-server.wasm

# Spin framework
spin up --from mcp-http-server.wasm

# Deploy to Fermyon Cloud
spin cloud deploy
```

## License

Apache-2.0