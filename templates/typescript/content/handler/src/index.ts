import { createTool, createHandler, z } from 'wasmcp';

// Define your tools using factory functions
export const echoTool = createTool({
  name: 'echo',
  description: 'Echo a message back to the user',
  schema: z.object({
    message: z.string().min(1).describe('Message to echo back')
  }),
  execute: async (args) => {
    return `Echo: ${args.message}`;
  }
});

// Weather tool that demonstrates async HTTP requests with fetch
export const weatherTool = createTool({
  name: 'weather',
  description: 'Get current weather for a location using Open-Meteo API',
  schema: z.object({
    location: z.string().describe('City name to get weather for')
  }),
  execute: async (args) => {
    try {
      // First, geocode the location
      const geocodingUrl = `https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(args.location)}&count=1`;
      const geocodingResponse = await fetch(geocodingUrl);
      const geocodingData = await geocodingResponse.json() as any;

      if (!geocodingData.results?.[0]) {
        return `Location '${args.location}' not found`;
      }

      const { latitude, longitude, name } = geocodingData.results[0];

      // Now fetch the weather data
      const weatherUrl = `https://api.open-meteo.com/v1/forecast?latitude=${latitude}&longitude=${longitude}&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code`;
      const weatherResponse = await fetch(weatherUrl);
      const weatherData = await weatherResponse.json() as any;

      const conditions = getWeatherCondition(weatherData.current.weather_code);

      return `Weather in ${name}:
Temperature: ${weatherData.current.temperature_2m}°C (feels like ${weatherData.current.apparent_temperature}°C)
Conditions: ${conditions}
Humidity: ${weatherData.current.relative_humidity_2m}%
Wind: ${weatherData.current.wind_speed_10m} km/h`;
    } catch (error) {
      return `Error fetching weather: ${error instanceof Error ? error.message : 'Unknown error'}`;
    }
  }
});

// Helper function to decode weather conditions
function getWeatherCondition(code: number): string {
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
    77: 'Snow grains',
    80: 'Slight rain showers',
    81: 'Moderate rain showers',
    82: 'Violent rain showers',
    85: 'Slight snow showers',
    86: 'Heavy snow showers',
    95: 'Thunderstorm',
    96: 'Thunderstorm with slight hail',
    99: 'Thunderstorm with heavy hail',
  };
  return conditions[code] || 'Unknown';
}

// Export all tools for testing
export const tools = [echoTool, weatherTool];

// Export the handler implementation for componentize-js
export const handler = createHandler({
  tools
});