# Weather Tools (TypeScript)

A TypeScript/JavaScript example demonstrating MCP tools with outbound HTTP requests and concurrent operations. Uses the Open-Meteo public API to fetch real-time weather data.

## Overview

The weather-ts component shows how to:

- **Build TypeScript components**: Use `componentize-js` to create WebAssembly components
- **Make HTTP requests**: Fetch data from external APIs using standard `fetch()`
- **Concurrent operations**: Use `Promise.all()` for parallel API calls
- **Schema validation**: Validate inputs and API responses with Zod
- **Tool annotations**: Mark tools as read-only and idempotent

This example demonstrates integrating real-world APIs into MCP tools.

## Tools Provided

| Tool | Description | Arguments |
|------|-------------|-----------|
| `get_weather` | Get current weather for one city | `location: string` |
| `multi_weather` | Get weather for multiple cities concurrently | `cities: string[] (max 3)` |

Both tools include annotations:
```typescript
{
  readOnlyHint: true,    // Doesn't modify state
  idempotentHint: true,  // Same inputs → same output
}
```

## Quick Start

```bash
# Install dependencies
npm install

# Build the component
npm run build
# Creates: dist/weather.wasm

# Compose into MCP server
make compose
# Creates: mcp-server.wasm

# Run with Spin
spin up

# In another terminal, test the tools
wasmcp mcp call-tool get_weather '{"location":"San Francisco"}'

wasmcp mcp call-tool multi_weather '{"cities":["Tokyo","London","Paris"]}'
```

## Building

### Prerequisites

```bash
# Install Node.js and npm
# Requires Node.js 18+ for fetch() support

# Install dependencies
npm install
```

### Build Process

The build happens in stages:

```bash
# 1. Generate TypeScript types from WIT files
npm run types
# Creates: src/generated/

# 2. Type check (optional)
npm run typecheck

# 3. Bundle with Webpack
npm run bundle
# Creates: build/bundled.js

# 4. Componentize into WebAssembly
npm run componentize
# Creates: dist/weather.wasm

# Or run all steps at once:
npm run build
```

## Implementation Guide

### 1. Define Your Component World

```wit
// wit/world.wit
package example:weather-ts;

world weather {
    // Import server-handler for MessageContext
    import wasmcp:mcp-v20250618/server-handler;

    // Import server-io for notifications
    import wasmcp:mcp-v20250618/server-io;

    // Export tools capability
    export wasmcp:mcp-v20250618/tools;
}
```

### 2. Generate TypeScript Types

```bash
npm run types
```

This creates TypeScript type definitions in `src/generated/` based on your WIT files.

### 3. Implement the Tools Interface

```typescript
import * as z from 'zod';
import type {
  ListToolsRequest,
  ListToolsResult,
  CallToolRequest,
  CallToolResult,
  Tool,
} from 'wasmcp:mcp-v20250618/mcp@0.1.7';
import type { RequestCtx } from 'wasmcp:mcp-v20250618/tools@0.1.7';

// Define input schema with Zod
const GetWeatherSchema = z.object({
  location: z.string().describe('City name to get weather for'),
});

type GetWeatherArgs = z.infer<typeof GetWeatherSchema>;

function listTools(
  _ctx: RequestCtx,
  _request: ListToolsRequest
): ListToolsResult {
  const tools: Tool[] = [
    {
      name: 'get_weather',
      inputSchema: JSON.stringify(z.toJSONSchema(GetWeatherSchema)),
      options: {
        title: 'Get Weather',
        description: 'Get current weather for a location using Open-Meteo API',
        annotations: {
          readOnlyHint: true,
          idempotentHint: true,
        },
      },
    },
  ];

  return { tools };
}

async function callTool(
  ctx: RequestCtx,
  request: CallToolRequest
): Promise<CallToolResult | undefined> {
  switch (request.name) {
    case 'get_weather':
      return await handleGetWeather(request.arguments);
    default:
      return undefined; // Tool not handled
  }
}

export const tools = {
  listTools,
  callTool,
};
```

### 4. Making HTTP Requests

Use standard `fetch()` API (available in wasi-http):

```typescript
async function getWeatherForCity(city: string): Promise<string> {
  // Step 1: Geocode the city name to coordinates
  const geoUrl = `https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(city)}&count=1`;
  const geoResponse = await fetch(geoUrl);

  if (!geoResponse.ok) {
    throw new Error(`Geocoding failed with status: ${geoResponse.status}`);
  }

  const geoData = GeocodingResponseSchema.parse(await geoResponse.json());

  if (!geoData.results || geoData.results.length === 0) {
    throw new Error(`Location '${city}' not found`);
  }

  const location = geoData.results[0];

  // Step 2: Fetch weather data for coordinates
  const weatherUrl =
    `https://api.open-meteo.com/v1/forecast?` +
    `latitude=${location.latitude}&longitude=${location.longitude}` +
    `&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code`;

  const weatherResponse = await fetch(weatherUrl);

  if (!weatherResponse.ok) {
    throw new Error(`Weather API failed with status: ${weatherResponse.status}`);
  }

  const weatherData = WeatherResponseSchema.parse(await weatherResponse.json());

  return formatWeatherOutput(location, weatherData);
}
```

**Key points**:
- Use `fetch()` directly - it's provided by wasi-http
- Handle errors gracefully
- Validate API responses with Zod schemas
- Always check `response.ok` before parsing

### 5. Concurrent Operations

Use `Promise.all()` for parallel API calls:

```typescript
async function handleMultiWeather(args?: string): Promise<CallToolResult> {
  const parsed: MultiWeatherArgs = MultiWeatherSchema.parse(JSON.parse(args));

  // Fetch weather for all cities concurrently
  const results = await Promise.all(
    parsed.cities.map(async (city) => {
      try {
        return await getWeatherForCity(city);
      } catch (error) {
        return `Error fetching weather for ${city}: ${error.message}`;
      }
    })
  );

  const output = results.join('\n\n');
  return textResult(output);
}
```

**Benefits**:
- Multiple API calls execute in parallel
- Faster than sequential requests
- Handles individual failures gracefully

### 6. Schema Validation with Zod

Define schemas for both inputs and API responses:

```typescript
import * as z from 'zod';

// Input validation
const GetWeatherSchema = z.object({
  location: z.string().describe('City name to get weather for'),
});

const MultiWeatherSchema = z.object({
  cities: z
    .array(z.string())
    .max(3)
    .describe('List of city names (max 3)'),
});

// API response validation
const WeatherResponseSchema = z.object({
  current: z.object({
    temperature_2m: z.number(),
    apparent_temperature: z.number(),
    relative_humidity_2m: z.number(),
    wind_speed_10m: z.number(),
    weather_code: z.number(),
  }),
});

// Usage
const parsed: GetWeatherArgs = GetWeatherSchema.parse(JSON.parse(args));
const weatherData = WeatherResponseSchema.parse(await response.json());
```

**Benefits**:
- Catch invalid inputs early
- Type-safe API responses
- Generate JSON Schema for MCP tool definitions
- Runtime validation

### 7. Error Handling

```typescript
async function handleGetWeather(args?: string): Promise<CallToolResult> {
  try {
    if (!args) {
      return errorResult('Arguments are required');
    }

    const parsed: GetWeatherArgs = GetWeatherSchema.parse(JSON.parse(args));
    const weather = await getWeatherForCity(parsed.location);
    return textResult(weather);
  } catch (error) {
    if (error instanceof z.ZodError) {
      return errorResult(`Invalid arguments: ${error.message}`);
    }
    return errorResult(
      `Error fetching weather: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

function errorResult(message: string): CallToolResult {
  return {
    content: [{
      tag: 'text',
      val: {
        text: { tag: 'text', val: message },
      },
    }],
    isError: true,
  };
}
```

### 8. Sending Notifications

Send progress updates during long operations:

```typescript
import { sendMessage } from 'wasmcp:mcp-v20250618/server-io@0.1.7';
import type {
  ServerMessage,
  ServerNotification,
  LoggingMessageNotification,
} from 'wasmcp:mcp-v20250618/mcp@0.1.7';

async function callTool(
  ctx: RequestCtx,
  request: CallToolRequest
): Promise<CallToolResult | undefined> {
  const log = (message: string) => {
    if (ctx.clientStream) {
      const notification: ServerNotification = {
        tag: 'log',
        val: {
          data: message,
          level: 'info',
          logger: 'weather-tools',
        } as LoggingMessageNotification,
      };
      const serverMessage: ServerMessage = {
        tag: 'notification',
        val: notification,
      };
      sendMessage(ctx.clientStream, serverMessage, ctx.frame);
    }
  };

  log('Fetching weather data...');
  const result = await handleGetWeather(request.arguments);
  log('Weather data retrieved successfully');

  return result;
}
```

## Build Configuration

### webpack.config.js

```javascript
module.exports = {
  entry: './src/index.ts',
  target: 'webworker',
  module: {
    rules: [
      {
        test: /\.ts$/,
        use: 'ts-loader',
        exclude: /node_modules/,
      },
    ],
  },
  resolve: {
    extensions: ['.ts', '.js'],
  },
  output: {
    filename: 'bundled.js',
    path: path.resolve(__dirname, 'build'),
    library: {
      type: 'module',
    },
  },
  experiments: {
    outputModule: true,
  },
};
```

### tsconfig.json

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ES2022",
    "lib": ["ES2022"],
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist", "build"]
}
```

## Testing

### With wasmcp CLI

```bash
# Start server
spin up

# Initialize session
wasmcp mcp initialize

# Get weather for one city
wasmcp mcp call-tool get_weather '{"location":"Tokyo"}'

# Get weather for multiple cities
wasmcp mcp call-tool multi_weather '{"cities":["New York","London","Sydney"]}'
```

### With curl

```bash
# Initialize session
SESSION_ID=$(curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -D - \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' \
  | grep -i "mcp-session-id" | cut -d' ' -f2 | tr -d '\r')

# Call get_weather
curl -X POST http://localhost:3000/mcp \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_weather","arguments":"{\"location\":\"Paris\"}"}}'
```

## Integration with Claude Code

Add to Claude Code MCP server configuration:

```json
{
  "mcpServers": {
    "weather": {
      "command": "spin",
      "args": [
        "up",
        "--listen",
        "127.0.0.1:3000",
        "--from",
        "/path/to/weather-ts"
      ]
    }
  }
}
```

Claude Code can then use the weather tools:

```
User: What's the weather like in Tokyo?
Claude: I'll check the weather for you.
[calls get_weather tool with {"location": "Tokyo"}]
Claude: The current weather in Tokyo is...
```

## API Information

This example uses the [Open-Meteo API](https://open-meteo.com/), which is:

- **Free**: No API key required
- **Open source**: CC BY 4.0 license
- **Real-time**: Current weather conditions
- **Global**: Coverage for cities worldwide

**Attribution**: Weather data by [Open-Meteo.com](https://open-meteo.com/)

### Rate Limits

Open-Meteo free tier allows:
- 10,000 API calls per day
- 5,000 per hour
- No commercial usage

For production use with higher limits, consider their commercial API.

## TypeScript Best Practices

### 1. Type Safety

Use generated types from WIT files:

```typescript
import type {
  CallToolResult,
  Tool,
  ErrorCode,
} from 'wasmcp:mcp-v20250618/mcp@0.1.7';
import type { RequestCtx } from 'wasmcp:mcp-v20250618/tools@0.1.7';

// Fully typed function signatures
function callTool(
  ctx: RequestCtx,
  request: CallToolRequest
): Promise<CallToolResult | undefined> {
  // TypeScript ensures type safety
}
```

### 2. Schema-Driven Development

Define schemas first, derive types:

```typescript
const MyToolSchema = z.object({
  input: z.string(),
  count: z.number().min(1).max(100),
});

type MyToolArgs = z.infer<typeof MyToolSchema>;

// Use in both runtime validation and type checking
const parsed: MyToolArgs = MyToolSchema.parse(JSON.parse(args));
```

### 3. Error Handling Patterns

```typescript
async function safeApiCall<T>(
  fn: () => Promise<T>,
  errorContext: string
): Promise<T> {
  try {
    return await fn();
  } catch (error) {
    throw new Error(
      `${errorContext}: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// Usage
const weather = await safeApiCall(
  () => getWeatherForCity(city),
  'Failed to fetch weather'
);
```

## Files

```
weather-ts/
├── package.json          # Node.js dependencies and scripts
├── tsconfig.json         # TypeScript configuration
├── webpack.config.js     # Webpack bundling configuration
├── Makefile              # Build and compose targets
├── README.md             # This file
├── spin.toml             # Spin runtime configuration
├── wit/
│   ├── deps/             # WIT dependencies
│   ├── deps.lock
│   ├── deps.toml
│   └── world.wit        # Component world definition
├── src/
│   ├── index.ts         # Tool implementation
│   └── generated/       # Generated types (created by build)
├── build/
│   └── bundled.js       # Webpack output (created by build)
└── dist/
    └── weather.wasm     # Final component (created by build)
```

## Troubleshooting

### Build Errors

**Problem**: `Cannot find module 'wasmcp:mcp-v20250618/mcp@0.1.7'`

**Solution**: Run `npm run types` to generate type definitions

**Problem**: Webpack errors about module resolution

**Solution**: Ensure `tsconfig.json` has `"moduleResolution": "bundler"`

### Runtime Errors

**Problem**: `fetch is not defined`

**Solution**: Ensure your component imports `wasi:http/outgoing-handler` in WIT

**Problem**: HTTP requests timeout

**Solution**: Check network connectivity and API availability

## Related Examples

- **calculator-rs** - Basic tools capability in Rust
- **strings-py** - Tools in Python
- **todo-list-auth** - Tools with authorization
- **routing-config** - Filter-middleware patterns

## Related Documentation

- [ComponentizeJS](https://github.com/bytecodealliance/ComponentizeJS)
- [Zod](https://zod.dev/)
- [Open-Meteo API](https://open-meteo.com/en/docs)
- [WebAssembly Component Model](https://component-model.bytecodealliance.org/)
