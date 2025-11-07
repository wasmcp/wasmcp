/**
 * Weather Tools Capability
 *
 * A tools capability that provides weather information using the Open-Meteo
 * public API. Demonstrates outbound HTTP requests, concurrent operations, and notifications.
 */

import * as z from 'zod';
import type {
  ListToolsRequest,
  ListToolsResult,
  CallToolRequest,
  CallToolResult,
  Tool,
  ServerMessage,
  ServerNotification,
  LoggingMessageNotification,
  LogLevel,
} from 'wasmcp:mcp-v20250618/mcp@0.1.5';
import type { RequestCtx } from 'wasmcp:mcp-v20250618/tools@0.1.5';
import { sendMessage } from 'wasmcp:mcp-v20250618/server-io@0.1.5';

// Tool input schemas
const GetWeatherSchema = z.object({
  location: z.string().describe('City name to get weather for'),
});

const MultiWeatherSchema = z.object({
  cities: z
    .array(z.string())
    .max(3)
    .describe('List of city names (max 3)'),
});

type GetWeatherArgs = z.infer<typeof GetWeatherSchema>;
type MultiWeatherArgs = z.infer<typeof MultiWeatherSchema>;

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

  return { tools };
}

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
          level: 'info' as LogLevel,
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

  switch (request.name) {
    case 'get_weather':
      return await handleGetWeather(request.arguments);
    case 'multi_weather':
      log(`Fetching weather concurrently for ${request.arguments}`);
      return await handleMultiWeather(request.arguments);
    default:
      return undefined; // We don't handle this tool
  }
}

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

async function handleMultiWeather(args?: string): Promise<CallToolResult> {
  try {
    if (!args) {
      return errorResult('Arguments are required');
    }

    const parsed: MultiWeatherArgs = MultiWeatherSchema.parse(JSON.parse(args));

    if (parsed.cities.length === 0) {
      return errorResult('No cities provided');
    }

    // Fetch weather for all cities concurrently
    const results = await Promise.all(
      parsed.cities.map(async (city) => {
        try {
          return await getWeatherForCity(city);
        } catch (error) {
          return `Error fetching weather for ${city}: ${error instanceof Error ? error.message : String(error)}`;
        }
      })
    );

    const output = `=== Weather Results ===

${results.join('\n\n')}

=== All requests completed ===`;

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

// Weather API integration
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
  // Geocode the location
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

  // Fetch the weather
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

function textResult(text: string): CallToolResult {
  return {
    content: [{
      tag: 'text',
      val: {
        text: { tag: 'text', val: text },
      },
    }],
    isError: false,
  };
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

export const tools = {
  listTools,
  callTool,
};
