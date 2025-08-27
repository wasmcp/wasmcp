mod bindings;
#[macro_use]
mod helpers;

use helpers::{Tool, ToolResult, McpError, ErrorCode, IntoToolResult, text_result};
use serde::Deserialize;
use serde_json::Value;
use spin_sdk::http::{Request, send};
use futures::future::join_all;

pub struct Component;

struct EchoTool;

impl Tool for EchoTool {
    const NAME: &'static str = "echo";
    const DESCRIPTION: &'static str = "Echo a message back to the user";
    
    fn input_schema() -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to echo"
                }
            },
            "required": ["message"]
        })
    }
    
    async fn execute(args: Value) -> Result<ToolResult, McpError> {
        let message = args.get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError {
                code: ErrorCode::InvalidParams,
                message: "Missing required field: message".to_string(),
                data: None,
            })?;
            
        Ok(format!("Echo: {}", message).into_result())
    }
}

// Weather tool with HTTP request
struct WeatherTool;

impl Tool for WeatherTool {
    const NAME: &'static str = "get_weather";
    const DESCRIPTION: &'static str = "Get current weather for a location";
    
    fn input_schema() -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name to get weather for"
                }
            },
            "required": ["location"]
        })
    }
    
    async fn execute(args: Value) -> Result<ToolResult, McpError> {
        let location = args.get("location")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError {
                code: ErrorCode::InvalidParams,
                message: "Missing required field: location".to_string(),
                data: None,
            })?;
        
        match get_weather_for_city(location).await {
            Ok(weather) => Ok(text_result(weather)),
            Err(e) => Ok(text_result(format!("Error fetching weather: {}", e)))
        }
    }
}

// Multi-weather tool with concurrent requests
struct MultiWeatherTool;

impl Tool for MultiWeatherTool {
    const NAME: &'static str = "multi_weather";
    const DESCRIPTION: &'static str = "Get weather for multiple cities concurrently";
    
    fn input_schema() -> Value {
        serde_json::json!({
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
    }
    
    async fn execute(args: Value) -> Result<ToolResult, McpError> {
        let cities = args.get("cities")
            .and_then(|v| v.as_array())
            .ok_or_else(|| McpError {
                code: ErrorCode::InvalidParams,
                message: "Missing or invalid 'cities' field".to_string(),
                data: None,
            })?;
        
        let city_names: Vec<String> = cities.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        
        if city_names.is_empty() {
            return Err(McpError {
                code: ErrorCode::InvalidParams,
                message: "No valid city names provided".to_string(),
                data: None,
            });
        }
        
        // Execute concurrent weather fetches
        let futures = city_names.iter().map(|city| {
            async move {
                let result = get_weather_for_city(city).await;
                (city.clone(), result)
            }
        });
        
        let results = join_all(futures).await;
        
        // Format results
        let mut output = String::from("=== Concurrent Weather Results ===\n\n");
        
        for (city, result) in results {
            match result {
                Ok(weather) => {
                    output.push_str(&weather);
                    output.push_str("\n\n");
                },
                Err(e) => {
                    output.push_str(&format!("Error fetching weather for {}: {}\n\n", city, e));
                }
            }
        }
        
        output.push_str("=== All requests completed concurrently ===");
        
        Ok(text_result(output))
    }
}

// Helper function to fetch weather for a single city
async fn get_weather_for_city(location: &str) -> Result<String, String> {
    // First, geocode the location
    let geocoding_url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1",
        urlencoding::encode(location)
    );
    
    let geo_response: spin_sdk::http::Response = send(Request::get(&geocoding_url)).await
        .map_err(|e| format!("Failed to geocode location: {:?}", e))?;
    
    let geo_body = geo_response.body().to_vec();
    let geo_data: GeocodingResponse = serde_json::from_slice(&geo_body)
        .map_err(|e| format!("Failed to parse geocoding response: {}", e))?;
    
    let location_data = geo_data.results.first()
        .ok_or_else(|| format!("Location '{}' not found", location))?;
    
    // Now fetch the weather
    let weather_url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code",
        location_data.latitude, location_data.longitude
    );
    
    let weather_response: spin_sdk::http::Response = send(Request::get(&weather_url)).await
        .map_err(|e| format!("Failed to fetch weather: {:?}", e))?;
    
    let weather_body = weather_response.body().to_vec();
    let weather_data: WeatherResponse = serde_json::from_slice(&weather_body)
        .map_err(|e| format!("Failed to parse weather response: {}", e))?;
    
    let conditions = get_weather_condition(weather_data.current.weather_code);
    
    Ok(format!(
        "Weather in {}, {}:\n\
        Temperature: {:.1}°C (feels like {:.1}°C)\n\
        Conditions: {}\n\
        Humidity: {}%\n\
        Wind: {:.1} km/h",
        location_data.name,
        location_data.country,
        weather_data.current.temperature_2m,
        weather_data.current.apparent_temperature,
        conditions,
        weather_data.current.relative_humidity_2m,
        weather_data.current.wind_speed_10m
    ))
}

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
        61 => "Slight rain",
        63 => "Moderate rain",
        65 => "Heavy rain",
        71 => "Slight snow",
        73 => "Moderate snow",
        75 => "Heavy snow",
        77 => "Snow grains",
        80 => "Slight rain showers",
        81 => "Moderate rain showers",
        82 => "Violent rain showers",
        85 => "Slight snow showers",
        86 => "Heavy snow showers",
        95 => "Thunderstorm",
        96 => "Thunderstorm with slight hail",
        99 => "Thunderstorm with heavy hail",
        _ => "Unknown"
    }
}

// Response types for API calls
#[derive(Deserialize)]
struct GeocodingResponse {
    results: Vec<LocationData>,
}

#[derive(Deserialize)]
struct LocationData {
    latitude: f64,
    longitude: f64,
    name: String,
    country: String,
}

#[derive(Deserialize)]
struct WeatherResponse {
    current: CurrentWeather,
}

#[derive(Deserialize)]
struct CurrentWeather {
    temperature_2m: f64,
    apparent_temperature: f64,
    relative_humidity_2m: i32,
    wind_speed_10m: f64,
    weather_code: i32,
}

// Generate the tool handlers using our macro
// This creates handle_list_tools and handle_call_tool functions
lazy_static::lazy_static! {
    static ref TOOL_HANDLERS: (
        fn(bindings::fastertools::mcp::tools::ListToolsRequest) -> Result<bindings::fastertools::mcp::tools::ListToolsResponse, bindings::fastertools::mcp::types::McpError>,
        fn(bindings::fastertools::mcp::tools::CallToolRequest) -> Result<bindings::fastertools::mcp::tools::ToolResult, bindings::fastertools::mcp::types::McpError>
    ) = register_tools!(
        EchoTool,
        WeatherTool,
        MultiWeatherTool
    );
}

// Implement the core interface for initialization
impl bindings::exports::fastertools::mcp::core::Guest for Component {
    fn handle_initialize(_request: bindings::fastertools::mcp::session::InitializeRequest) 
        -> Result<bindings::fastertools::mcp::session::InitializeResponse, bindings::fastertools::mcp::types::McpError> {
        Ok(bindings::fastertools::mcp::session::InitializeResponse {
            protocol_version: "2025-06-18".to_string(),
            capabilities: bindings::fastertools::mcp::session::ServerCapabilities {
                tools: Some(bindings::fastertools::mcp::session::ToolsCapability { 
                    list_changed: Some(false) 
                }),
                // Not implementing resources or prompts - null components will handle these
                resources: None,
                prompts: None,
                experimental: None,
                logging: None,
                completions: None,
            },
            server_info: bindings::fastertools::mcp::session::ImplementationInfo {
                name: "{{project-name | snake_case}}".to_string(),
                version: "0.1.0".to_string(),
                title: Some("{{project-name | kebab_case}} Handler".to_string()),
            },
            instructions: None,
            meta: None,
        })
    }
    
    fn handle_initialized() -> Result<(), bindings::fastertools::mcp::types::McpError> {
        Ok(())
    }
    
    fn handle_ping() -> Result<(), bindings::fastertools::mcp::types::McpError> {
        Ok(())
    }
    
    fn handle_shutdown() -> Result<(), bindings::fastertools::mcp::types::McpError> {
        Ok(())
    }
}

// Implement the tool handler interface - just delegate to the generated functions
impl bindings::exports::fastertools::mcp::tool_handler::Guest for Component {
    fn handle_list_tools(request: bindings::fastertools::mcp::tools::ListToolsRequest) 
        -> Result<bindings::fastertools::mcp::tools::ListToolsResponse, bindings::fastertools::mcp::types::McpError> {
        (TOOL_HANDLERS.0)(request)
    }
    
    fn handle_call_tool(request: bindings::fastertools::mcp::tools::CallToolRequest) 
        -> Result<bindings::fastertools::mcp::tools::ToolResult, bindings::fastertools::mcp::types::McpError> {
        (TOOL_HANDLERS.1)(request)
    }
}

// Export the component
bindings::export!(Component with_types_in bindings);