/**
 * Transparent MCP provider implementation for weather-ts.
 *
 * This implementation uses WIT bindings directly as the SDK, without
 * abstraction layers.
 */

// Import WIT-generated types directly
import type {
  InitializeRequest,
  InitializeResponse,
  ProtocolVersion,
  ServerCapabilities,
  ImplementationInfo,
  ToolsCapability,
} from './generated/interfaces/fastertools-mcp-core-types.js';

import type {
  Tool,
  ToolResult,
  ListToolsRequest,
  ListToolsResponse,
  CallToolRequest,
} from './generated/interfaces/fastertools-mcp-tool-types.js';

import type {
  ContentBlock,
  ContentBlockText,
  TextContent,
  BaseMetadata,
} from './generated/interfaces/fastertools-mcp-types.js';

import type {
  ProviderAuthConfig,
} from './generated/interfaces/fastertools-mcp-authorization-types.js';

// -------------------------------------------------------------------------
// Core Capabilities Implementation
// -------------------------------------------------------------------------

export const coreCapabilities = {
  handleInitialize(_request: InitializeRequest): InitializeResponse {
    const toolsCapability: ToolsCapability = {
      listChanged: undefined,
    };

    const capabilities: ServerCapabilities = {
      tools: toolsCapability,
      experimental: undefined,
      logging: undefined,
      completions: undefined,
      prompts: undefined,
      resources: undefined,
    };

    const serverInfo: ImplementationInfo = {
      name: '{{project-name}}',
      version: '0.1.0',
      title: '{{project-name}} Server',
    };

    return {
      protocolVersion: 'v20250618' as ProtocolVersion,
      capabilities,
      serverInfo,
      instructions: 'A TypeScript MCP server providing weather tools',
      meta: undefined,
    };
  },

  handleInitialized(): void {
    // No-op
  },

  handlePing(): void {
    // No-op
  },

  handleShutdown(): void {
    // No-op
  },

  getAuthConfig(): ProviderAuthConfig | undefined {
    // Uncomment and configure to enable OAuth authentication:
    // return {
    //   expectedIssuer: 'https://your-auth-domain.example.com',
    //   expectedAudiences: ['your-client-id'],
    //   jwksUri: 'https://your-auth-domain.example.com/oauth2/jwks',
    //   policy: undefined,
    //   policyData: undefined,
    // };
    return undefined;
  },

  jwksCacheGet(_jwksUri: string): string | undefined {
    // Optional: Implement JWKS caching
    return undefined;
  },

  jwksCacheSet(_jwksUri: string, _jwks: string): void {
    // Optional: Implement JWKS caching
  },
};

// -------------------------------------------------------------------------
// Tools Capabilities Implementation
// -------------------------------------------------------------------------

export const toolsCapabilities = {
  handleListTools(_request: ListToolsRequest): ListToolsResponse {
    const tools: Tool[] = [
      {
        base: {
          name: 'echo',
          title: 'echo',
        } as BaseMetadata,
        description: 'Echo a message back to the user',
        inputSchema: JSON.stringify({
          type: 'object',
          properties: {
            message: {
              type: 'string',
              description: 'The message to echo',
            },
          },
          required: ['message'],
        }),
        outputSchema: undefined,
        annotations: undefined,
        meta: undefined,
      },
      {
        base: {
          name: 'get_weather',
          title: 'get_weather',
        } as BaseMetadata,
        description: 'Get current weather for a location',
        inputSchema: JSON.stringify({
          type: 'object',
          properties: {
            location: {
              type: 'string',
              description: 'City name to get weather for',
            },
          },
          required: ['location'],
        }),
        outputSchema: undefined,
        annotations: undefined,
        meta: undefined,
      },
      {
        base: {
          name: 'multi_weather',
          title: 'multi_weather',
        } as BaseMetadata,
        description: 'Get weather for multiple cities concurrently',
        inputSchema: JSON.stringify({
          type: 'object',
          properties: {
            cities: {
              type: 'array',
              description: 'List of city names (max 5)',
              items: {
                type: 'string',
              },
            },
          },
          required: ['cities'],
        }),
        outputSchema: undefined,
        annotations: undefined,
        meta: undefined,
      },
    ];

    return {
      tools,
      nextCursor: undefined,
      meta: undefined,
    };
  },

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
  },
};

// -------------------------------------------------------------------------
// Tool Implementations
// -------------------------------------------------------------------------

interface EchoArgs {
  message: string;
}

async function handleEcho(args?: string): Promise<ToolResult> {
  try {
    const parsedArgs: EchoArgs = args ? JSON.parse(args) : { message: '' };
    return textResult(`Echo: ${parsedArgs.message}`);
  } catch (error) {
    return errorResult(`Invalid arguments: ${error instanceof Error ? error.message : String(error)}`);
  }
}

interface WeatherArgs {
  location: string;
}

async function handleGetWeather(args?: string): Promise<ToolResult> {
  try {
    const parsedArgs: WeatherArgs = args ? JSON.parse(args) : { location: '' };
    const weather = await getWeatherForCity(parsedArgs.location);
    return textResult(weather);
  } catch (error) {
    return errorResult(`Error fetching weather: ${error instanceof Error ? error.message : String(error)}`);
  }
}

interface MultiWeatherArgs {
  cities: string[];
}

async function handleMultiWeather(args?: string): Promise<ToolResult> {
  try {
    const parsedArgs: MultiWeatherArgs = args ? JSON.parse(args) : { cities: [] };

    if (parsedArgs.cities.length === 0) {
      return errorResult('No cities provided');
    }

    if (parsedArgs.cities.length > 5) {
      return errorResult('Maximum 5 cities allowed');
    }

    // Execute all requests concurrently
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

    let output = '=== Weather Results ===\n\n';
    for (const result of results) {
      output += result;
      output += '\n';
    }
    output += '=== All requests completed ===';

    return textResult(output);
  } catch (error) {
    return errorResult(`Invalid arguments: ${error instanceof Error ? error.message : String(error)}`);
  }
}

// -------------------------------------------------------------------------
// Weather API Functions
// -------------------------------------------------------------------------

interface GeocodingResponse {
  results?: Array<{
    name: string;
    country: string;
    latitude: number;
    longitude: number;
  }>;
}

interface WeatherResponse {
  current: {
    temperature_2m: number;
    apparent_temperature: number;
    relative_humidity_2m: number;
    wind_speed_10m: number;
    weather_code: number;
  };
}

async function getWeatherForCity(city: string): Promise<string> {
  // First, geocode the location
  const geoUrl = `https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(city)}&count=1`;

  const geoResponse = await fetch(geoUrl);
  if (!geoResponse.ok) {
    throw new Error(`Geocoding failed with status: ${geoResponse.status}`);
  }

  const geoData = (await geoResponse.json()) as GeocodingResponse;

  if (!geoData.results || geoData.results.length === 0) {
    throw new Error(`Location '${city}' not found`);
  }

  const location = geoData.results[0];
  if (!location) {
    throw new Error(`No location data found for '${city}'`);
  }

  // Now fetch the weather
  const weatherUrl =
    `https://api.open-meteo.com/v1/forecast?` +
    `latitude=${location.latitude}&longitude=${location.longitude}` +
    `&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code`;

  const weatherResponse = await fetch(weatherUrl);
  if (!weatherResponse.ok) {
    throw new Error(`Weather API failed with status: ${weatherResponse.status}`);
  }

  const weatherData = (await weatherResponse.json()) as WeatherResponse;

  const condition = weatherCondition(weatherData.current.weather_code);

  return `Weather in ${location.name}, ${location.country}:
Temperature: ${weatherData.current.temperature_2m.toFixed(1)}°C (feels like ${weatherData.current.apparent_temperature.toFixed(1)}°C)
Conditions: ${condition}
Humidity: ${weatherData.current.relative_humidity_2m}%
Wind: ${weatherData.current.wind_speed_10m.toFixed(1)} km/h`;
}

function weatherCondition(code: number): string {
  const conditions: Record<number, string> = {
    0: 'Clear sky',
    1: 'Mainly clear',
    2: 'Partly cloudy',
    3: 'Overcast',
    45: 'Foggy',
    48: 'Depositing rime fog',
    51: 'Light drizzle',
    53: 'Moderate drizzle',
    55: 'Dense drizzle',
    61: 'Slight rain',
    63: 'Moderate rain',
    65: 'Heavy rain',
    71: 'Slight snow fall',
    73: 'Moderate snow fall',
    75: 'Heavy snow fall',
    80: 'Slight rain showers',
    81: 'Moderate rain showers',
    82: 'Violent rain showers',
    85: 'Slight snow showers',
    86: 'Heavy snow showers',
    95: 'Thunderstorm',
    96: 'Thunderstorm with slight hail',
    99: 'Thunderstorm with heavy hail',
  };
  return conditions[code] ?? 'Unknown';
}

// -------------------------------------------------------------------------
// Helper Functions
// -------------------------------------------------------------------------

function textResult(text: string): ToolResult {
  const textContent: TextContent = {
    text,
    annotations: undefined,
    meta: undefined,
  };

  const contentBlock: ContentBlockText = {
    tag: 'text',
    val: textContent,
  };

  return {
    content: [contentBlock as ContentBlock],
    structuredContent: undefined,
    isError: false,
    meta: undefined,
  };
}

function errorResult(message: string): ToolResult {
  const textContent: TextContent = {
    text: message,
    annotations: undefined,
    meta: undefined,
  };

  const contentBlock: ContentBlockText = {
    tag: 'text',
    val: textContent,
  };

  return {
    content: [contentBlock as ContentBlock],
    structuredContent: undefined,
    isError: true,
    meta: undefined,
  };
}