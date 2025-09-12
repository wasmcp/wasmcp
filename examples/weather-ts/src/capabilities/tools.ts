/**
 * Tools capability implementation for weather-ts MCP server.
 * 
 * This module provides tools for weather information retrieval, including
 * concurrent fetching for multiple cities. TypeScript/JavaScript's native
 * async/await and Promise.all() provide natural concurrency support.
 */

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

/**
 * List available tools.
 * 
 * Tools are defined inline as an array. The inputSchema is a JSON string
 * because WIT's json-object type is represented as a string in the bindings.
 * This is similar to Python's approach but different from Rust's json! macro.
 */
export function listTools(_request: ListToolsRequest): ListToolsResult {
  const tools: Tool[] = [
    {
      name: 'echo',
      title: 'echo',
      description: 'Echo a message back to the user',
      icons: undefined,
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
    },
    {
      name: 'get_weather',
      title: 'get_weather',
      description: 'Get current weather for a location',
      icons: undefined,
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
    },
    {
      name: 'multi_weather',
      title: 'multi_weather',
      description: 'Get weather for multiple cities concurrently',
      icons: undefined,
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
    const args = request.arguments ? JSON.parse(request.arguments) : {};
    
    switch (request.name) {
      case 'echo':
        return await handleEcho(args);
      
      case 'get_weather':
        return await handleGetWeather(args);
      
      case 'multi_weather':
        return await handleMultiWeather(args);
      
      default:
        return errorResult(`Unknown tool: ${request.name}`);
    }
  } catch (error) {
    return errorResult(
      `Error executing ${request.name}: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// -------------------------------------------------------------------------
// Tool Implementations
// -------------------------------------------------------------------------

interface EchoArgs {
  message: string;
}

async function handleEcho(args: any): Promise<CallToolResult> {
  try {
    const parsedArgs: EchoArgs = typeof args === 'string' ? JSON.parse(args) : args;
    return textResult(`Echo: ${parsedArgs.message}`);
  } catch (error) {
    return errorResult(`Invalid arguments: ${error instanceof Error ? error.message : String(error)}`);
  }
}

interface WeatherArgs {
  location: string;
}

async function handleGetWeather(args: any): Promise<CallToolResult> {
  try {
    const parsedArgs: WeatherArgs = typeof args === 'string' ? JSON.parse(args) : args;
    const weather = await getWeatherForCity(parsedArgs.location);
    return textResult(weather);
  } catch (error) {
    return errorResult(`Error fetching weather: ${error instanceof Error ? error.message : String(error)}`);
  }
}

interface MultiWeatherArgs {
  cities: string[];
}

async function handleMultiWeather(args: any): Promise<CallToolResult> {
  try {
    const parsedArgs: MultiWeatherArgs = typeof args === 'string' ? JSON.parse(args) : args;
    
    if (parsedArgs.cities.length === 0) {
      return errorResult('No cities provided');
    }
    
    if (parsedArgs.cities.length > 5) {
      return errorResult('Maximum 5 cities allowed');
    }
    
    // JavaScript's Promise.all() provides natural concurrency.
    // Unlike Go which needs wasihttp.RequestsConcurrently() or Python
    // which needs PollLoop, JavaScript's async model works naturally with jco.
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
  
  // JavaScript's fetch API works in the WebAssembly environment through
  // the Component Model's HTTP imports. jco handles the bridging between
  // JavaScript's fetch and WASI HTTP, providing seamless async support!
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