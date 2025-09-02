//! MCP provider implementation for weather-rs.
//!
//! This module provides tools accessible via the Model Context Protocol.

#![warn(missing_docs)]
// Allow unsafe in generated bindings only
#![allow(unsafe_code)]

#[allow(warnings)]
mod bindings;
#[macro_use]
mod helpers;

use bindings::fastertools::mcp::authorization::ProviderAuthConfig;

use futures::future::join_all;
use helpers::{parse_args, text_result, IntoToolResult, McpError, Tool, ToolResult};
use serde::Deserialize;
use serde_json::json;
use spin_sdk::http::{send, Request, Response};

// ==============================================================================
// AUTHENTICATION CONFIGURATION
// ==============================================================================

/// OAuth 2.0 authentication configuration.
/// 
/// To enable authentication:
/// 1. Uncomment the auth_config() function below
/// 2. Replace the placeholder values with your actual OAuth provider details
/// 3. Run `make build` to rebuild with authentication enabled
/// 
/// To disable authentication:
/// - Comment out the auth_config() function or have it return None
pub fn auth_config() -> Option<ProviderAuthConfig> {
    // Uncomment and configure the lines below to enable OAuth 2.0 authentication:
    /*
    Some(ProviderAuthConfig {
        expected_issuer: "https://your-auth-domain.example.com".to_string(),
        expected_audiences: vec!["your-client-id".to_string()],
        jwks_uri: "https://your-auth-domain.example.com/oauth2/jwks".to_string(),
        policy: None,  // Optional: Add Rego policy as a string for additional authorization rules
        policy_data: None,  // Optional: Add policy data as JSON string
    })
    */
    
    // Authentication disabled by default - return None for no auth
    None
}

// ==============================================================================
// MCP PROVIDER IMPLEMENTATION
// ==============================================================================

/// The main component struct required by the WIT bindings.
pub struct Component;

/// Arguments for the echo tool.
#[derive(Deserialize)]
struct EchoArgs {
    message: String,
}

/// Echo tool - echoes a message back to the user.
struct EchoTool;

impl Tool for EchoTool {
    const NAME: &'static str = "echo";
    const DESCRIPTION: &'static str = "Echo a message back to the user";

    fn input_schema() -> String {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to echo"
                }
            },
            "required": ["message"]
        })
        .to_string()
    }

    async fn execute(args: Option<String>) -> Result<ToolResult, McpError> {
        let args: EchoArgs = parse_args(&args)?;
        Ok(format!("Echo: {}", args.message).into_result())
    }
}

/// Arguments for the weather tool.
#[derive(Deserialize)]
struct WeatherArgs {
    location: String,
}

/// Weather tool - fetches current weather for a single location.
struct WeatherTool;

impl Tool for WeatherTool {
    const NAME: &'static str = "get_weather";
    const DESCRIPTION: &'static str = "Get current weather for a location";

    fn input_schema() -> String {
        json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name to get weather for"
                }
            },
            "required": ["location"]
        })
        .to_string()
    }

    async fn execute(args: Option<String>) -> Result<ToolResult, McpError> {
        let args: WeatherArgs = parse_args(&args)?;

        match get_weather_for_city(&args.location).await {
            Ok(weather) => Ok(text_result(weather)),
            Err(e) => Ok(format!("Error fetching weather: {e}").into_error()),
        }
    }
}

/// Arguments for the multi-weather tool.
#[derive(Deserialize)]
struct MultiWeatherArgs {
    cities: Vec<String>,
}

/// Multi-weather tool - fetches weather for multiple cities concurrently.
struct MultiWeatherTool;

impl Tool for MultiWeatherTool {
    const NAME: &'static str = "multi_weather";
    const DESCRIPTION: &'static str = "Get weather for multiple cities concurrently";

    fn input_schema() -> String {
        json!({
            "type": "object",
            "properties": {
                "cities": {
                    "type": "array",
                    "description": "List of cities to get weather for",
                    "items": {
                        "type": "string"
                    },
                    "minItems": 1,
                    "maxItems": 5
                }
            },
            "required": ["cities"]
        })
        .to_string()
    }

    async fn execute(args: Option<String>) -> Result<ToolResult, McpError> {
        let args: MultiWeatherArgs = parse_args(&args)?;

        if args.cities.is_empty() {
            return Ok("No cities provided".into_error());
        }

        if args.cities.len() > 5 {
            return Ok("Maximum 5 cities allowed".into_error());
        }

        // Create futures for all cities
        let futures = args.cities.iter().map(|city| {
            let city = city.clone();
            Box::pin(async move {
                match get_weather_for_city(&city).await {
                    Ok(weather) => format!("{weather}\n"),
                    Err(e) => format!("Error fetching weather for {city}: {e}\n"),
                }
            })
        });

        // Execute all requests concurrently
        let results = join_all(futures).await;

        let mut output = String::from("=== Weather Results ===\n\n");
        for result in results {
            output.push_str(&result);
            output.push('\n');
        }
        output.push_str("=== All requests completed ===");

        Ok(text_result(output))
    }
}

/// Geocoding response structure.
#[derive(Debug, Deserialize)]
struct GeocodingResponse {
    results: Option<Vec<GeocodingResult>>,
}

/// Individual geocoding result.
#[derive(Debug, Deserialize)]
struct GeocodingResult {
    latitude: f64,
    longitude: f64,
    name: String,
    country: String,
}

/// Weather API response structure.
#[derive(Debug, Deserialize)]
struct WeatherResponse {
    current: CurrentWeather,
}

/// Current weather data.
#[derive(Debug, Deserialize)]
struct CurrentWeather {
    temperature_2m: f64,
    apparent_temperature: f64,
    relative_humidity_2m: i32,
    wind_speed_10m: f64,
    weather_code: i32,
}

/// Fetches weather data for a single city.
///
/// # Errors
/// Returns an error if geocoding or weather fetching fails.
async fn get_weather_for_city(location: &str) -> Result<String, String> {
    // First, geocode the location
    let geocoding_url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1",
        urlencoding::encode(location)
    );

    let geocoding_req = Request::get(&geocoding_url).build();

    let geocoding_resp: Response = send(geocoding_req)
        .await
        .map_err(|e| format!("Geocoding request failed: {e}"))?;

    if *geocoding_resp.status() != 200 {
        return Err(format!(
            "Geocoding failed with status: {}",
            geocoding_resp.status()
        ));
    }

    let geocoding_body = geocoding_resp
        .body()
        .to_vec();

    let geocoding_data: GeocodingResponse = serde_json::from_slice(&geocoding_body)
        .map_err(|e| format!("Failed to parse geocoding response: {e}"))?;

    let loc = geocoding_data
        .results
        .and_then(|r| r.into_iter().next())
        .ok_or_else(|| format!("Location '{location}' not found"))?;

    // Now fetch weather data
    let weather_url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code",
        loc.latitude, loc.longitude
    );

    let weather_req = Request::get(&weather_url).build();

    let weather_resp: Response = send(weather_req)
        .await
        .map_err(|e| format!("Weather request failed: {e}"))?;

    if *weather_resp.status() != 200 {
        return Err(format!(
            "Weather API failed with status: {}",
            weather_resp.status()
        ));
    }

    let weather_body = weather_resp
        .body()
        .to_vec();

    let weather_data: WeatherResponse = serde_json::from_slice(&weather_body)
        .map_err(|e| format!("Failed to parse weather response: {e}"))?;

    let conditions = get_weather_condition(weather_data.current.weather_code);

    Ok(format!(
        "Weather in {}, {}:\n\
         Temperature: {:.1}°C (feels like {:.1}°C)\n\
         Conditions: {}\n\
         Humidity: {}%\n\
         Wind: {:.1} km/h",
        loc.name,
        loc.country,
        weather_data.current.temperature_2m,
        weather_data.current.apparent_temperature,
        conditions,
        weather_data.current.relative_humidity_2m,
        weather_data.current.wind_speed_10m
    ))
}

/// Converts weather code to human-readable condition.
fn get_weather_condition(code: i32) -> &'static str {
    match code {
        0 => "Clear sky",
        1 => "Mainly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 => "Foggy",
        48 => "Depositing rime fog",
        51 => "Light drizzle",
        53 => "Moderate drizzle",
        55 => "Dense drizzle",
        56 => "Light freezing drizzle",
        57 => "Dense freezing drizzle",
        61 => "Slight rain",
        63 => "Moderate rain",
        65 => "Heavy rain",
        66 => "Light freezing rain",
        67 => "Heavy freezing rain",
        71 => "Slight snow fall",
        73 => "Moderate snow fall",
        75 => "Heavy snow fall",
        77 => "Snow grains",
        80 => "Slight rain showers",
        81 => "Moderate rain showers",
        82 => "Violent rain showers",
        85 => "Slight snow showers",
        86 => "Heavy snow showers",
        95 => "Thunderstorm",
        96 => "Thunderstorm with slight hail",
        99 => "Thunderstorm with heavy hail",
        _ => "Unknown",
    }
}

// Register all tools with the MCP provider
register_tools!(EchoTool, WeatherTool, MultiWeatherTool);