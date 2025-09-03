/**
 * {{project-name | kebab_case}} MCP Provider
 * 
 * {{project-description}}
 * 
 * Includes example tools:
 * - echo: Echo a message back
 * - get_weather: Get weather for a single location
 * - multi_weather: Get weather for multiple locations concurrently
 */

import { createTool, createHandler } from './helpers.js';

// ==============================================================================
// AUTHENTICATION CONFIGURATION
// ==============================================================================

/**
 * OAuth 2.0 authentication configuration.
 * 
 * To enable authentication:
 * 1. Uncomment the authConfig object below
 * 2. Replace the placeholder values with your actual OAuth provider details
 * 3. Run `make build` to rebuild with authentication enabled
 * 
 * To disable authentication:
 * - Leave authConfig as null (default)
 */
const authConfig = null;

// Uncomment and configure the lines below to enable OAuth 2.0 authentication:
/*
const authConfig = {
    expectedIssuer: 'https://your-auth-domain.example.com',
    expectedAudiences: ['your-client-id'],
    jwksUri: 'https://your-auth-domain.example.com/oauth2/jwks',
    policy: null,      // Optional: Add Rego policy as a string for additional authorization rules
    policyData: null   // Optional: Add policy data as JSON string
};
*/

// ==============================================================================
// MCP TOOLS IMPLEMENTATION
// ==============================================================================

// Define the echo tool
export const echoTool = createTool({
    name: 'echo',
    description: 'Echo a message back to the user',
    schema: {
        type: 'object',
        properties: {
            message: {
                type: 'string',
                description: 'The message to echo'
            }
        },
        required: ['message']
    },
    execute: async (args) => {
        if (!args.message) {
            throw new Error('Missing required field: message');
        }
        return `Echo: ${args.message}`;
    }
});

// Define the single weather tool
export const weatherTool = createTool({
    name: 'get_weather',
    description: 'Get current weather for a location',
    schema: {
        type: 'object',
        properties: {
            location: {
                type: 'string',
                description: 'City name to get weather for'
            }
        },
        required: ['location']
    },
    execute: async (args) => {
        if (!args.location) {
            throw new Error('Missing required field: location');
        }
        return await getWeatherForCity(args.location);
    }
});

// Define the multi-weather tool
export const multiWeatherTool = createTool({
    name: 'multi_weather',
    description: 'Get weather for multiple cities concurrently',
    schema: {
        type: 'object',
        properties: {
            cities: {
                type: 'array',
                description: 'List of cities to get weather for',
                items: {
                    type: 'string'
                },
                minItems: 1,
                maxItems: 5
            }
        },
        required: ['cities']
    },
    execute: async (args) => {
        const cities = args.cities;
        
        if (!cities || !Array.isArray(cities) || cities.length === 0) {
            throw new Error('Missing or invalid cities field');
        }
        
        if (cities.length > 5) {
            throw new Error('Maximum 5 cities allowed');
        }
        
        // Execute concurrent weather fetches
        const results = await Promise.all(
            cities.map(async city => {
                try {
                    const weather = await getWeatherForCity(city);
                    return { city, success: true, data: weather };
                } catch (error) {
                    return { city, success: false, error: error.message };
                }
            })
        );
        
        // Format results
        let output = "";
        
        for (const result of results) {
            if (result.success) {
                output += result.data + "\n\n";
            } else {
                output += `Error fetching weather for ${result.city}: ${result.error}\n\n`;
            }
        }
                
        return output;
    }
});

// Helper function to get weather for a single city
async function getWeatherForCity(location) {
    try {
        // First, geocode the location
        const geocodingUrl = `https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(location)}&count=1`;
        const geoResponse = await fetch(geocodingUrl);
        const geoData = await geoResponse.json();
        
        if (!geoData.results || geoData.results.length === 0) {
            throw new Error(`Location '${location}' not found`);
        }
        
        const locationData = geoData.results[0];
        
        // Now fetch the weather
        const weatherUrl = `https://api.open-meteo.com/v1/forecast?` +
            `latitude=${locationData.latitude}&longitude=${locationData.longitude}` +
            `&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code`;
        
        const weatherResponse = await fetch(weatherUrl);
        const weatherData = await weatherResponse.json();
        
        const conditions = getWeatherCondition(weatherData.current.weather_code);
        
        return `Weather in ${locationData.name}, ${locationData.country}:
Temperature: ${weatherData.current.temperature_2m.toFixed(1)}°C (feels like ${weatherData.current.apparent_temperature.toFixed(1)}°C)
Conditions: ${conditions}
Humidity: ${weatherData.current.relative_humidity_2m}%
Wind: ${weatherData.current.wind_speed_10m.toFixed(1)} km/h`;
    } catch (error) {
        throw new Error(`Failed to fetch weather for ${location}: ${error.message}`);
    }
}

// Weather condition descriptions based on WMO codes
function getWeatherCondition(code) {
    const conditions = {
        0: 'Clear sky',
        1: 'Mainly clear',
        2: 'Partly cloudy',
        3: 'Overcast',
        45: 'Foggy',
        48: 'Depositing rime fog',
        51: 'Light drizzle',
        53: 'Moderate drizzle',
        55: 'Dense drizzle',
        56: 'Light freezing drizzle',
        57: 'Dense freezing drizzle',
        61: 'Slight rain',
        63: 'Moderate rain',
        65: 'Heavy rain',
        66: 'Light freezing rain',
        67: 'Heavy freezing rain',
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
        99: 'Thunderstorm with heavy hail'
    };
    return conditions[code] || 'Unknown';
}

// ==============================================================================
// MCP SERVER CONFIGURATION
// ==============================================================================

// Create the handler with both core and tools capabilities
const handler = createHandler({
    tools: [echoTool, weatherTool, multiWeatherTool],
    serverInfo: {
        name: '{{project-name | kebab_case}}',
        version: '0.1.0',
        instructions: '{{project-description}}'
    },
    authConfig: authConfig  // Uses the auth configuration defined at the top
});

// Export both capabilities as expected by the WIT interface
export const coreCapabilities = handler.coreCapabilities;
export const toolsCapabilities = handler.toolsCapabilities;