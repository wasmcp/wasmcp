/**
 * Type definitions for MCP tools
 */

export interface ToolSchema {
    type: 'object';
    properties: Record<string, any>;
    required?: string[];
}

export interface ToolDefinition {
    name: string;
    description: string;
    schema: ToolSchema;
    execute: (args: any) => Promise<string>;
}

export interface WeatherData {
    location: string;
    temperature: string;
    conditions: string;
    humidity: string;
    wind: string;
}

export interface GeocodingResult {
    results?: Array<{
        name: string;
        country: string;
        latitude: number;
        longitude: number;
    }>;
}

export interface WeatherApiResponse {
    current: {
        temperature_2m: number;
        apparent_temperature: number;
        relative_humidity_2m: number;
        wind_speed_10m: number;
        weather_code: number;
    };
}

export interface CityWeatherResult {
    city: string;
    success: boolean;
    data?: string;
    error?: string;
}