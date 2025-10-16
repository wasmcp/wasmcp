/**
 * Weather Tools Capability
 *
 * A simple MCP tools capability that provides weather information using the
 * Open-Meteo public API. Demonstrates outbound HTTP requests using fetch.
 *
 * Uses Zod v4 for schema definition and automatic JSON Schema conversion.
 */

import * as z from 'zod';
import type {
  ListToolsRequest,
  ListToolsResult,
  CallToolRequest,
  CallToolResult,
  ClientContext,
  Tool,
  ContentBlock,
  TextContent,
} from './generated/interfaces/wasmcp-mcp-protocol.js';

// =========================================================================
// Tool Schemas (Zod)
// =========================================================================

const GetWeatherSchema = z.object({
  location: z.string().describe('City name to get weather for'),
});

const MultiWeatherSchema = z.object({
  cities: z
    .array(z.string())
    .max(3)
    .describe('List of city names (max 3)'),
});

// Type inference for runtime validation
// eslint-disable-next-line @typescript-eslint/no-unused-vars
type GetWeatherArgs = z.infer<typeof GetWeatherSchema>;
// eslint-disable-next-line @typescript-eslint/no-unused-vars
type MultiWeatherArgs = z.infer<typeof MultiWeatherSchema>;

// =========================================================================
// Tools Capability Interface Implementation
// =========================================================================

/**
 * List all tools provided by this capability
 */
function listTools(
  _request: ListToolsRequest,
  _client: ClientContext
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
    {
      name: 'multi_weather',
      inputSchema: JSON.stringify(z.toJSONSchema(MultiWeatherSchema)),
      options: {
        title: 'Multi Weather',
        description: 'Get weather for multiple cities concurrently',
        annotations: {
          readOnlyHint: true,
          idempotentHint: true,
        },
      },
    },
  ];

  return {
    tools,
  };
}

/**
 * Execute a tool call
 *
 * Returns Some(result) if we handle this tool, None otherwise
 */
async function callTool(
  request: CallToolRequest,
  _client: ClientContext
): Promise<CallToolResult | undefined> {
  try {
    switch (request.name) {
      case 'get_weather':
        return await handleGetWeather(request.arguments);
      case 'multi_weather':
        return await handleMultiWeather(request.arguments);
      default:
        // Return undefined to indicate we don't handle this tool
        // Middleware will delegate to next capability
        return undefined;
    }
  } catch (error) {
    return errorResult(
      `Error executing ${request.name}: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// =========================================================================
// Tool Implementations
// =========================================================================

async function handleGetWeather(args?: string): Promise<CallToolResult> {
  try {
    if (!args) {
      return errorResult('Arguments are required');
    }

    const parsedArgs = GetWeatherSchema.parse(JSON.parse(args));
    const weather = await getWeatherForCity(parsedArgs.location);
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

async function handleMultiWeather(args?: string): Promise<CallToolResult> {
  try {
    if (!args) {
      return errorResult('Arguments are required');
    }

    const parsedArgs = MultiWeatherSchema.parse(JSON.parse(args));

    if (parsedArgs.cities.length === 0) {
      return errorResult('No cities provided');
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
    if (error instanceof z.ZodError) {
      return errorResult(`Invalid arguments: ${error.message}`);
    }
    return errorResult(
      `Error processing request: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// =========================================================================
// Weather API Functions
// =========================================================================

const GeocodingResponseSchema = z.object({
  results: z
    .array(
      z.object({
        name: z.string(),
        country: z.string(),
        latitude: z.number(),
        longitude: z.number(),
      })
    )
    .optional(),
});

const WeatherResponseSchema = z.object({
  current: z.object({
    temperature_2m: z.number(),
    apparent_temperature: z.number(),
    relative_humidity_2m: z.number(),
    wind_speed_10m: z.number(),
    weather_code: z.number(),
  }),
});

async function getWeatherForCity(city: string): Promise<string> {
  // First, geocode the location
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

  const weatherData = WeatherResponseSchema.parse(await weatherResponse.json());

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

// =========================================================================
// Helper Functions
// =========================================================================

function textResult(text: string): CallToolResult {
  const textContent: TextContent = {
    text: { tag: 'text', val: text },
  };

  const contentBlock: ContentBlock = {
    tag: 'text',
    val: textContent,
  };

  return {
    content: [contentBlock],
    isError: false,
  };
}

function errorResult(message: string): CallToolResult {
  const textContent: TextContent = {
    text: { tag: 'text', val: message },
  };

  const contentBlock: ContentBlock = {
    tag: 'text',
    val: textContent,
  };

  return {
    content: [contentBlock],
    isError: true,
  };
}

// =========================================================================
// Export the capability interface
// =========================================================================

export const toolsCapability = {
  listTools,
  callTool,
};
