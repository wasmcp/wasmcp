/**
 * JavaScript MCP Weather Handler
 * 
 * This implements the same tools as the Rust example:
 * - echo: Echo a message back
 * - get_weather: Get weather for a single location
 * - multi_weather: Get weather for multiple locations concurrently
 */

// Tool definitions with their metadata
const toolDefinitions = [
    {
        base: {
            name: "echo",
            title: "Echo Tool"
        },
        description: "Echo a message back to the user",
        inputSchema: JSON.stringify({
            type: "object",
            properties: {
                message: {
                    type: "string",
                    description: "The message to echo"
                }
            },
            required: ["message"]
        }),
        outputSchema: null,
        annotations: null,
        meta: null
    },
    {
        base: {
            name: "get_weather",
            title: "Weather Tool"
        },
        description: "Get current weather for a location",
        inputSchema: JSON.stringify({
            type: "object",
            properties: {
                location: {
                    type: "string",
                    description: "City name to get weather for"
                }
            },
            required: ["location"]
        }),
        outputSchema: null,
        annotations: null,
        meta: null
    },
    {
        base: {
            name: "multi_weather",
            title: "Multi Weather Tool"
        },
        description: "Get weather for multiple cities concurrently",
        inputSchema: JSON.stringify({
            type: "object",
            properties: {
                cities: {
                    type: "array",
                    description: "List of cities to get weather for",
                    items: {
                        type: "string"
                    },
                    minItems: 1,
                    maxItems: 5
                }
            },
            required: ["cities"]
        }),
        outputSchema: null,
        annotations: null,
        meta: null
    }
];

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
        0: "Clear sky",
        1: "Mainly clear",
        2: "Partly cloudy",
        3: "Overcast",
        45: "Foggy",
        48: "Foggy",
        51: "Drizzle",
        53: "Drizzle",
        55: "Drizzle",
        56: "Freezing drizzle",
        57: "Freezing drizzle",
        61: "Rain",
        63: "Rain",
        65: "Rain",
        66: "Freezing rain",
        67: "Freezing rain",
        71: "Snow",
        73: "Snow",
        75: "Snow",
        77: "Snow grains",
        80: "Rain showers",
        81: "Rain showers",
        82: "Rain showers",
        85: "Snow showers",
        86: "Snow showers",
        95: "Thunderstorm",
        96: "Thunderstorm with hail",
        99: "Thunderstorm with hail"
    };
    return conditions[code] || "Unknown";
}

// Tool execution logic
async function executeTool(name, args) {
    const parsedArgs = args ? JSON.parse(args) : {};
    
    switch (name) {
        case "echo": {
            const message = parsedArgs.message;
            if (!message) {
                throw new Error("Missing required field: message");
            }
            return {
                content: [{
                    tag: 'text',
                    val: {
                        text: `Echo: ${message}`,
                        annotations: null,
                        meta: null
                    }
                }],
                structuredContent: null,
                isError: false,
                meta: null
            };
        }
        
        case "get_weather": {
            const location = parsedArgs.location;
            if (!location) {
                throw new Error("Missing required field: location");
            }
            
            try {
                const weather = await getWeatherForCity(location);
                return {
                    content: [{
                        tag: 'text',
                        val: {
                            text: weather,
                            annotations: null,
                            meta: null
                        }
                    }],
                    structuredContent: null,
                    isError: false,
                    meta: null
                };
            } catch (error) {
                return {
                    content: [{
                        tag: 'text',
                        val: {
                            text: `Error fetching weather: ${error.message}`,
                            annotations: null,
                            meta: null
                        }
                    }],
                    structuredContent: null,
                    isError: true,
                    meta: null
                };
            }
        }
        
        case "multi_weather": {
            const cities = parsedArgs.cities;
            if (!cities || !Array.isArray(cities) || cities.length === 0) {
                throw new Error("Missing or invalid 'cities' field");
            }
            
            if (cities.length > 5) {
                throw new Error("Maximum 5 cities allowed");
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
            let output = "=== Concurrent Weather Results ===\n\n";
            
            for (const result of results) {
                if (result.success) {
                    output += result.data + "\n\n";
                } else {
                    output += `Error fetching weather for ${result.city}: ${result.error}\n\n`;
                }
            }
            
            output += "=== All requests completed concurrently ===";
            
            return {
                content: [{
                    tag: 'text',
                    val: {
                        text: output,
                        annotations: null,
                        meta: null
                    }
                }],
                structuredContent: null,
                isError: false,
                meta: null
            };
        }
        
        default:
            throw new Error(`Unknown tool: ${name}`);
    }
}


// Export the MCP tool handler interface
// jco expects the interface to be exported as an object with the interface name
export const toolHandler = {
    handleListTools(request) {
        console.log('handleListTools called with:', JSON.stringify(request));
        const response = {
            tools: toolDefinitions,
            nextCursor: null,
            meta: null
        };
        console.log('handleListTools returning:', JSON.stringify(response));
        return response;
    },
    
    async handleCallTool(request) {
        // Return the result directly, let jco handle the Result wrapping
        const result = await executeTool(request.name, request.arguments);
        return result;
    }
};