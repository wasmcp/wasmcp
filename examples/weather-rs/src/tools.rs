// Tools implementation using the Guest trait pattern.
// This demonstrates how Rust handles async operations in the Component Model.
use crate::bindings::exports::wasmcp::mcp::tools::Guest as ToolsGuest;
use crate::bindings::wasmcp::mcp::{
    authorization_types::AuthContext,
    mcp_types::{ContentBlock, ErrorCode, McpError, TextContent},
    tools_types::{CallToolRequest, CallToolResult, ListToolsRequest, ListToolsResult, Tool},
};
use crate::Component;
use futures::future::join_all;
use serde::Deserialize;
use serde_json::json;
// Spin SDK provides WebAssembly-compatible HTTP client.
// Unlike standard reqwest or hyper, spin_sdk works in the WebAssembly environment
// by using the Component Model's HTTP imports.
use spin_sdk::http::{send, Request, Response};

impl ToolsGuest for Component {
    /// List available tools.
    ///
    /// Tools are defined inline as a Vec, similar to Python's list approach.
    /// The json! macro creates JSON schemas that are converted to strings
    /// for WIT's json-object type.
    fn list_tools(_request: ListToolsRequest) -> Result<ListToolsResult, McpError> {
        let tools = vec![
            Tool {
                name: "echo".to_string(),
                title: Some("echo".to_string()),
                description: Some("Echo a message back to the user".to_string()),
                icons: None,
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
            },
            Tool {
                name: "get_weather".to_string(),
                title: Some("get_weather".to_string()),
                description: Some("Get current weather for a location".to_string()),
                icons: None,
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
            },
            Tool {
                name: "multi_weather".to_string(),
                title: Some("multi_weather".to_string()),
                description: Some("Get weather for multiple cities concurrently".to_string()),
                icons: None,
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
            },
        ];

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    /// Execute a tool with the given request.
    ///
    /// The _ctx parameter is Option<AuthContext>, naturally mapping to WIT's option<auth-context>.
    /// 
    /// Key pattern: spin_sdk::http::run() bridges async/sync:
    /// - Component Model exports are synchronous
    /// - spin_sdk::http::run() executes an async block to completion
    /// - This is similar to Python's PollLoop but more integrated
    fn call_tool(
        request: CallToolRequest,
        _ctx: Option<AuthContext>,
    ) -> Result<CallToolResult, McpError> {
        match request.name.as_str() {
            // Each tool handler runs in an async context via spin_sdk
            "echo" => spin_sdk::http::run(async move { handle_echo(request.arguments.as_ref()) }),
            "get_weather" => {
                spin_sdk::http::run(async move { handle_get_weather(request.arguments).await })
            }
            "multi_weather" => {
                spin_sdk::http::run(async move { handle_multi_weather(request.arguments).await })
            }
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

fn handle_echo(args: Option<&String>) -> Result<CallToolResult, McpError> {
    let args: EchoArgs = parse_args(args)?;
    Ok(text_result(format!("Echo: {}", args.message)))
}

#[derive(Deserialize)]
struct WeatherArgs {
    location: String,
}

async fn handle_get_weather(args: Option<String>) -> Result<CallToolResult, McpError> {
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

async fn handle_multi_weather(args: Option<String>) -> Result<CallToolResult, McpError> {
    let args: MultiWeatherArgs = parse_args(args.as_ref())?;

    if args.cities.is_empty() {
        return Ok(error_result("No cities provided".to_string()));
    }

    if args.cities.len() > 5 {
        return Ok(error_result("Maximum 5 cities allowed".to_string()));
    }

    // Concurrent HTTP in Rust WebAssembly:
    // Unlike Go which needs special handling (wasihttp.RequestsConcurrently),
    // Rust's async/await works naturally with futures::join_all.
    // The spin_sdk runtime handles the WebAssembly poll-based I/O,
    // similar to Python's PollLoop but fully integrated with Rust's async ecosystem.
    
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

    // Execute all requests concurrently - true parallelism via the host runtime
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

#[derive(Debug, Clone, Deserialize)]
struct GeocodingResult {
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Deserialize)]
struct WeatherResponse {
    current: CurrentWeather,
}

#[derive(Debug, Deserialize)]
struct CurrentWeather {
    time: String,
    temperature_2m: f64,
    apparent_temperature: f64,
    relative_humidity_2m: u32,
    wind_speed_10m: f64,
    wind_direction_10m: u32,
    weather_code: u32,
}

async fn get_weather_for_city(city: &str) -> Result<String, String> {
    // Get coordinates for the city
    let geocoding_url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=en&format=json",
        city
    );

    // spin_sdk::http::send() uses Component Model HTTP imports under the hood.
    // This is not a regular network call - it goes through the WebAssembly runtime
    // which handles the actual HTTP via the host's networking stack.
    let response: Response = send(Request::get(&geocoding_url)).await.map_err(|e| e.to_string())?;

    if *response.status() != 200 {
        return Err(format!("Geocoding API error: {}", response.status()));
    }

    let body = response.into_body();
    let geocoding: GeocodingResponse =
        serde_json::from_slice(&body).map_err(|e| format!("Failed to parse geocoding response: {e}"))?;

    let location = geocoding
        .results
        .and_then(|r| r.first().cloned())
        .ok_or_else(|| format!("Location '{}' not found", city))?;

    // Get weather for the coordinates
    let weather_url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,wind_direction_10m,weather_code&temperature_unit=celsius&wind_speed_unit=ms",
        location.latitude, location.longitude
    );

    let response: Response = send(Request::get(&weather_url)).await.map_err(|e| e.to_string())?;

    if *response.status() != 200 {
        return Err(format!("Weather API error: {}", response.status()));
    }

    let body = response.into_body();
    let weather: WeatherResponse =
        serde_json::from_slice(&body).map_err(|e| format!("Failed to parse weather response: {e}"))?;

    // Format the weather report
    Ok(format!(
        "🌍 Weather for {}:
📅 Time: {}
🌡️  Temperature: {:.1}°C (feels like {:.1}°C)
💧 Humidity: {}%
💨 Wind: {:.1} m/s from {}°
☁️  Condition: {}",
        city,
        weather.current.time,
        weather.current.temperature_2m,
        weather.current.apparent_temperature,
        weather.current.relative_humidity_2m,
        weather.current.wind_speed_10m,
        weather.current.wind_direction_10m,
        weather_code_to_string(weather.current.weather_code)
    ))
}

fn weather_code_to_string(code: u32) -> &'static str {
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

fn text_result(text: String) -> CallToolResult {
    CallToolResult {
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

fn error_result(message: String) -> CallToolResult {
    CallToolResult {
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