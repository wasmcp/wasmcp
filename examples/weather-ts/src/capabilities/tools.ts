/**
 * Tools capability implementation for weather-ts MCP server.
 * 
 * This module provides tools for weather information retrieval, including
 * concurrent fetching for multiple cities. TypeScript/JavaScript's native
 * async/await and Promise.all() provide natural concurrency support.
 */

import { z } from 'zod';
import type {
  ListToolsRequest,
  ListToolsResult,
  CallToolRequest,
  CallToolResult,
  Tool,
} from '../generated/interfaces/wasmcp-mcp-tools-types.js';
import type { 
  ContentBlock,
  TextContent,
} from '../generated/interfaces/wasmcp-mcp-mcp-types.js';
import type { AuthContext } from '../generated/interfaces/wasmcp-mcp-authorization-types.js';

// -------------------------------------------------------------------------
// Tool Schemas with Zod - Type-safe and idiomatic TypeScript
// -------------------------------------------------------------------------

const EchoSchema = z.object({
  message: z.string().describe('The message to echo'),
});
type EchoArgs = z.infer<typeof EchoSchema>;

const WeatherSchema = z.object({
  location: z.string().describe('City name to get weather for'),
});
type WeatherArgs = z.infer<typeof WeatherSchema>;

const MultiWeatherSchema = z.object({
  cities: z.array(z.string()).min(1).max(5).describe('List of city names (1-5)'),
});
type MultiWeatherArgs = z.infer<typeof MultiWeatherSchema>;

/**
 * Convert a Zod schema to JSON Schema for the WIT interface.
 * Zod v4 has built-in JSON Schema generation via z.toJSONSchema().
 * This gives us type safety AND proper schema generation!
 */
function zodToJsonSchema(schema: z.ZodType<any>): string {
  // Use Zod v4's built-in JSON Schema generation
  const jsonSchema = z.toJSONSchema(schema);
  return JSON.stringify(jsonSchema);
}

/**
 * List available tools.
 * 
 * Now using Zod schemas as the single source of truth for both
 * TypeScript types AND JSON Schema generation. Much more idiomatic!
 */
export function listTools(_request: ListToolsRequest): ListToolsResult {
  const tools: Tool[] = [
    {
      name: 'echo',
      title: 'echo',
      description: 'Echo a message back to the user',
      icons: undefined,
      inputSchema: zodToJsonSchema(EchoSchema),
      outputSchema: undefined,
      annotations: undefined,
    },
    {
      name: 'get_weather',
      title: 'get_weather',
      description: 'Get current weather for a location',
      icons: undefined,
      inputSchema: zodToJsonSchema(WeatherSchema),
      outputSchema: undefined,
      annotations: undefined,
    },
    {
      name: 'multi_weather',
      title: 'multi_weather',
      description: 'Get weather for multiple cities concurrently',
      icons: undefined,
      inputSchema: zodToJsonSchema(MultiWeatherSchema),
      outputSchema: undefined,
      annotations: undefined,
    },
  ];

  return {
    tools,
    nextCursor: undefined,
  };
}

/**
 * Execute a tool with the given request.
 * 
 * The context parameter is optional (AuthContext | undefined), mapping to
 * WIT's option<auth-context>. This allows authentication to be optional.
 * 
 * jco handles the async-to-sync bridge transparently,
 * allowing us to use native JavaScript async/await with fetch.
 */
export async function callTool(
  request: CallToolRequest,
  _context: AuthContext | undefined
): Promise<CallToolResult> {
  try {
    // Parse the arguments - they come as a JSON string per the WIT type
    const args = request.arguments ? JSON.parse(request.arguments) : {};
    
    switch (request.name) {
      case 'echo': {
        const validated = EchoSchema.parse(args);
        return await handleEcho(validated);
      }
      
      case 'get_weather': {
        const validated = WeatherSchema.parse(args);
        return await handleGetWeather(validated);
      }
      
      case 'multi_weather': {
        const validated = MultiWeatherSchema.parse(args);
        return await handleMultiWeather(validated);
      }
      
      default:
        return errorResult(`Unknown tool: ${request.name}`);
    }
  } catch (error) {
    if (error instanceof z.ZodError) {
      return errorResult(`Invalid arguments: ${error.issues.map((e) => `${e.path.join('.')}: ${e.message}`).join(', ')}`);
    }
    return errorResult(
      `Error executing ${request.name}: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// -------------------------------------------------------------------------
// Tool Implementations - Now with proper typed arguments!
// -------------------------------------------------------------------------

async function handleEcho(args: EchoArgs): Promise<CallToolResult> {
  return textResult(`Echo: ${args.message}`);
}

async function handleGetWeather(args: WeatherArgs): Promise<CallToolResult> {
  try {
    const weather = await getWeatherForCity(args.location);
    return textResult(weather);
  } catch (error) {
    return errorResult(`Error fetching weather: ${error instanceof Error ? error.message : String(error)}`);
  }
}

async function handleMultiWeather(args: MultiWeatherArgs): Promise<CallToolResult> {
  // Zod already validated the array length, so we know it's 1-5 cities
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
  
  let output = '=== Weather Results ===\n\n';
  for (const result of results) {
    output += result;
    output += '\n';
  }
  output += '=== All requests completed ===';
  
  return textResult(output);
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
    time: string;
    temperature_2m: number;
    apparent_temperature: number;
    relative_humidity_2m: number;
    wind_speed_10m: number;
    wind_direction_10m: number;
    weather_code: number;
  };
}

async function getWeatherForCity(city: string): Promise<string> {
  // Get coordinates for the city
  const geoUrl = `https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(city)}&count=1&language=en&format=json`;
  
  // fetch API through Component Model's HTTP imports
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
  
  // Get weather for the coordinates
  const weatherUrl = (
    `https://api.open-meteo.com/v1/forecast?` +
    `latitude=${location.latitude}&longitude=${location.longitude}` +
    `&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,wind_direction_10m,weather_code` +
    `&temperature_unit=celsius&wind_speed_unit=ms`
  );
  
  const weatherResponse = await fetch(weatherUrl);
  if (!weatherResponse.ok) {
    throw new Error(`Weather API failed with status: ${weatherResponse.status}`);
  }
  
  const weatherData = (await weatherResponse.json()) as WeatherResponse;
  
  return formatWeather(location.name, location.country, weatherData.current);
}

function formatWeather(
  city: string,
  country: string,
  current: WeatherResponse['current']
): string {
  return `🌍 Weather for ${city}, ${country}:
📅 Time: ${current.time}
🌡️  Temperature: ${current.temperature_2m.toFixed(1)}°C (feels like ${current.apparent_temperature.toFixed(1)}°C)
💧 Humidity: ${current.relative_humidity_2m}%
💨 Wind: ${current.wind_speed_10m.toFixed(1)} m/s from ${current.wind_direction_10m}°
☁️  Condition: ${weatherCodeToString(current.weather_code)}`;
}

function weatherCodeToString(code: number): string {
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

function textResult(text: string): CallToolResult {
  const textContent: TextContent = {
    text,
    annotations: undefined,
    meta: undefined,
  };
  
  // TypeScript's variant types from WIT are represented as tagged unions.
  // This is different from Rust's enum variants but similar to how
  // discriminated unions work in TypeScript.
  const contentBlock: ContentBlock = {
    tag: 'text',
    val: textContent,
  };
  
  return {
    content: [contentBlock],
    structuredContent: undefined,
    isError: false,
    meta: undefined,
  };
}

function errorResult(message: string): CallToolResult {
  const textContent: TextContent = {
    text: message,
    annotations: undefined,
    meta: undefined,
  };
  
  const contentBlock: ContentBlock = {
    tag: 'text',
    val: textContent,
  };
  
  return {
    content: [contentBlock],
    structuredContent: undefined,
    isError: true,
    meta: undefined,
  };
}