//! Transparent MCP provider implementation for weather-rs.
//!
//! This implementation uses WIT bindings directly as the SDK, without
//! abstraction layers.

#[allow(warnings)]
mod bindings;

use bindings::exports::fastertools::mcp::core_capabilities::Guest as CoreGuest;
use bindings::exports::fastertools::mcp::tools_capabilities::Guest as ToolsGuest;
use bindings::fastertools::mcp::{
    authorization_types::ProviderAuthConfig,
    core_types::{
        ImplementationInfo,
        InitializeRequest,
        InitializeResponse,
        ProtocolVersion,
        ServerCapabilities,
        ToolsCapability,
    },
    tool_types::{
        BaseMetadata,
        CallToolRequest,
        ListToolsRequest,
        ListToolsResponse,
        Tool,
        ToolResult,
    },
    types::{
        ContentBlock,
        ErrorCode,
        McpError,
        TextContent,
    },
};
use futures::future::join_all;
use serde::Deserialize;
use serde_json::json;
use spin_sdk::http::{
    Request,
    Response,
    send,
};

/// The main component struct required by the WIT bindings.
pub struct Component;

// -------------------------------------------------------------------------
// Core Capabilities Implementation
// -------------------------------------------------------------------------

impl CoreGuest for Component {
    fn handle_initialize(_request: InitializeRequest) -> Result<InitializeResponse, McpError> {
        Ok(InitializeResponse {
            protocol_version: ProtocolVersion::V20250618,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: None }),
                experimental: None,
                logging: None,
                completions: None,
                prompts: None,
                resources: None,
            },
            server_info: ImplementationInfo {
                name: "weather-rs".to_string(),
                version: "0.1.0".to_string(),
                title: Some("weather-rs Server".to_string()),
            },
            instructions: Some("A Rust MCP server providing weather tools".to_string()),
            meta: None,
        })
    }

    fn handle_initialized() -> Result<(), McpError> {
        Ok(())
    }

    fn handle_ping() -> Result<(), McpError> {
        Ok(())
    }

    fn handle_shutdown() -> Result<(), McpError> {
        Ok(())
    }

    fn get_auth_config() -> Option<ProviderAuthConfig> {
        // Uncomment and configure to enable OAuth authorization:
        // Some(ProviderAuthConfig {
        // expected_issuer: "https://your-auth-domain.example.com".to_string(),
        // expected_audiences: vec!["your-client-id".to_string()],
        // jwks_uri: "https://your-auth-domain.example.com/oauth2/jwks".to_string(),
        // policy: None,
        // policy_data: None,
        // })
        None
    }

    fn jwks_cache_get(_jwks_uri: String) -> Option<String> {
        // Optional: Implement JWKS caching
        None
    }

    fn jwks_cache_set(_jwks_uri: String, _jwks: String) {
        // Optional: Implement JWKS caching
    }
}

// -------------------------------------------------------------------------
// Tools Capabilities Implementation
// -------------------------------------------------------------------------

impl ToolsGuest for Component {
    fn handle_list_tools(_request: ListToolsRequest) -> Result<ListToolsResponse, McpError> {
        let tools = vec![
            Tool {
                base: BaseMetadata {
                    name: "echo".to_string(),
                    title: Some("echo".to_string()),
                },
                description: Some("Echo a message back to the user".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "The message to echo"
                        }
                    },
                    "required": ["message"]
                })
                .to_string(),
                output_schema: None,
                annotations: None,
                meta: None,
            },
            Tool {
                base: BaseMetadata {
                    name: "get_weather".to_string(),
                    title: Some("get_weather".to_string()),
                },
                description: Some("Get current weather for a location".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "City name to get weather for"
                        }
                    },
                    "required": ["location"]
                })
                .to_string(),
                output_schema: None,
                annotations: None,
                meta: None,
            },
            Tool {
                base: BaseMetadata {
                    name: "multi_weather".to_string(),
                    title: Some("multi_weather".to_string()),
                },
                description: Some("Get weather for multiple cities concurrently".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "cities": {
                            "type": "array",
                            "description": "List of city names (max 5)",
                            "items": {
                                "type": "string"
                            }
                        }
                    },
                    "required": ["cities"]
                })
                .to_string(),
                output_schema: None,
                annotations: None,
                meta: None,
            },
        ];

        Ok(ListToolsResponse {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    fn handle_call_tool(request: CallToolRequest) -> Result<ToolResult, McpError> {
        match request.name.as_str() {
            "echo" => spin_sdk::http::run(async move { handle_echo(request.arguments.as_ref()) }),
            "get_weather" => {
                spin_sdk::http::run(async move { handle_get_weather(request.arguments).await })
            },
            "multi_weather" => {
                spin_sdk::http::run(async move { handle_multi_weather(request.arguments).await })
            },
            _ => Err(McpError {
                code: ErrorCode::MethodNotFound,
                message: format!("Unknown tool: {}", request.name),
                data: None,
            }),
        }
    }
}

// -------------------------------------------------------------------------
// Tool Implementations
// -------------------------------------------------------------------------

#[derive(Deserialize)]
struct EchoArgs {
    message: String,
}

fn handle_echo(args: Option<&String>) -> Result<ToolResult, McpError> {
    let args: EchoArgs = parse_args(args)?;
    Ok(text_result(format!("Echo: {}", args.message)))
}

#[derive(Deserialize)]
struct WeatherArgs {
    location: String,
}

async fn handle_get_weather(args: Option<String>) -> Result<ToolResult, McpError> {
    let args: WeatherArgs = parse_args(args.as_ref())?;

    match get_weather_for_city(&args.location).await {
        Ok(weather) => Ok(text_result(weather)),
        Err(e) => Ok(error_result(format!("Error fetching weather: {e}"))),
    }
}

#[derive(Deserialize)]
struct MultiWeatherArgs {
    cities: Vec<String>,
}

async fn handle_multi_weather(args: Option<String>) -> Result<ToolResult, McpError> {
    let args: MultiWeatherArgs = parse_args(args.as_ref())?;

    if args.cities.is_empty() {
        return Ok(error_result("No cities provided".to_string()));
    }

    if args.cities.len() > 5 {
        return Ok(error_result("Maximum 5 cities allowed".to_string()));
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

// -------------------------------------------------------------------------
// Weather API Functions
// -------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GeocodingResponse {
    results: Option<Vec<GeocodingResult>>,
}

#[derive(Debug, Deserialize)]
struct GeocodingResult {
    latitude: f64,
    longitude: f64,
    name: String,
    country: String,
}

#[derive(Debug, Deserialize)]
struct WeatherResponse {
    current: CurrentWeather,
}

#[derive(Debug, Deserialize)]
struct CurrentWeather {
    temperature_2m: f64,
    apparent_temperature: f64,
    relative_humidity_2m: i32,
    wind_speed_10m: f64,
    weather_code: i32,
}

async fn get_weather_for_city(city: &str) -> Result<String, String> {
    // First, geocode the location
    let geo_url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1",
        urlencoding::encode(city)
    );

    let geo_request = Request::get(&geo_url)
        .header("User-Agent", "MCP-Weather-Server")
        .build();

    let geo_response: Response = send(geo_request)
        .await
        .map_err(|e| format!("Geocoding request failed: {e}"))?;

    if *geo_response.status() != 200 {
        return Err(format!(
            "Geocoding failed with status: {}",
            geo_response.status()
        ));
    }

    let geo_body = geo_response.body();
    let geo_data: GeocodingResponse = serde_json::from_slice(geo_body)
        .map_err(|e| format!("Failed to parse geocoding response: {e}"))?;

    let location = geo_data
        .results
        .and_then(|r| r.into_iter().next())
        .ok_or_else(|| format!("Location '{city}' not found"))?;

    // Now fetch the weather
    let weather_url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code",
        location.latitude, location.longitude
    );

    let weather_request = Request::get(&weather_url)
        .header("User-Agent", "MCP-Weather-Server")
        .build();

    let weather_response: Response = send(weather_request)
        .await
        .map_err(|e| format!("Weather request failed: {e}"))?;

    if *weather_response.status() != 200 {
        return Err(format!(
            "Weather API failed with status: {}",
            weather_response.status()
        ));
    }

    let weather_body = weather_response.body();
    let weather_data: WeatherResponse = serde_json::from_slice(weather_body)
        .map_err(|e| format!("Failed to parse weather response: {e}"))?;

    let condition = weather_condition(weather_data.current.weather_code);

    Ok(format!(
        "Weather in {}, {}:\nTemperature: {:.1}°C (feels like {:.1}°C)\nConditions: {}\nHumidity: \
         {}%\nWind: {:.1} km/h",
        location.name,
        location.country,
        weather_data.current.temperature_2m,
        weather_data.current.apparent_temperature,
        condition,
        weather_data.current.relative_humidity_2m,
        weather_data.current.wind_speed_10m
    ))
}

fn weather_condition(code: i32) -> &'static str {
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
        71 => "Slight snow fall",
        73 => "Moderate snow fall",
        75 => "Heavy snow fall",
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

// -------------------------------------------------------------------------
// Helper Functions
// -------------------------------------------------------------------------

fn parse_args<T: for<'a> Deserialize<'a>>(args: Option<&String>) -> Result<T, McpError> {
    let args_str = args.map_or("{}", String::as_str);
    serde_json::from_str(args_str).map_err(|e| McpError {
        code: ErrorCode::InvalidParams,
        message: format!("Failed to parse arguments: {e}"),
        data: None,
    })
}

fn text_result(text: String) -> ToolResult {
    ToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text,
            annotations: None,
            meta: None,
        })],
        structured_content: None,
        is_error: Some(false),
        meta: None,
    }
}

fn error_result(message: String) -> ToolResult {
    ToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: message,
            annotations: None,
            meta: None,
        })],
        structured_content: None,
        is_error: Some(true),
        meta: None,
    }
}

// Export the WIT bindings
bindings::export!(Component with_types_in bindings);
