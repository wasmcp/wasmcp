/**
 * Type definitions for MCP tools with strict typing
 */

export interface ToolSchema {
  readonly type: 'object';
  readonly properties: Record<string, unknown>;
  readonly required?: readonly string[];
}

export interface ToolDefinition<TArgs = Record<string, unknown>> {
  readonly name: string;
  readonly description: string;
  readonly schema: ToolSchema;
  readonly execute: (args: TArgs) => Promise<string>;
}

// Weather-specific types
export interface WeatherData {
  readonly location: string;
  readonly temperature: string;
  readonly conditions: string;
  readonly humidity: string;
  readonly wind: string;
}

export interface GeocodingResult {
  readonly results?: readonly {
    readonly name: string;
    readonly country: string;
    readonly latitude: number;
    readonly longitude: number;
  }[];
}

export interface WeatherApiResponse {
  readonly current: {
    readonly temperature_2m: number;
    readonly apparent_temperature: number;
    readonly relative_humidity_2m: number;
    readonly wind_speed_10m: number;
    readonly weather_code: number;
  };
}

export interface CityWeatherResult {
  readonly city: string;
  readonly success: boolean;
  readonly data?: string;
  readonly error?: string;
}

// Tool argument types - must extend Record<string, unknown> for generic constraint
export interface EchoArgs extends Record<string, unknown> {
  readonly message: string;
}

export interface WeatherArgs extends Record<string, unknown> {
  readonly location: string;
}

export interface MultiWeatherArgs extends Record<string, unknown> {
  readonly cities: readonly string[];
}
