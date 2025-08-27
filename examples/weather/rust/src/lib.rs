mod bindings;

use serde::Deserialize;
use bindings::exports::fastertools::mcp::{
    core::Guest as CoreGuest, 
    tool_handler::Guest as ToolGuest,
    resource_handler::Guest as ResourceGuest,
    prompt_handler::Guest as PromptGuest,
};
use bindings::fastertools::mcp::types;
use bindings::fastertools::mcp::tools;
use bindings::fastertools::mcp::session;
use bindings::fastertools::mcp::resources;
use bindings::fastertools::mcp::prompts;

struct Component;

// Implement core handlers (required)
impl CoreGuest for Component {
    fn handle_initialize(_request: session::InitializeRequest) -> Result<session::InitializeResponse, types::McpError> {
        Ok(session::InitializeResponse {
            protocol_version: "0.1.0".to_string(),
            capabilities: session::ServerCapabilities {
                tools: Some(session::ToolsCapability {
                    list_changed: Some(false),
                }),
                resources: None,  // We don't provide resources
                prompts: None,    // We don't provide prompts
                experimental: None,
                logging: None,
                completions: None,
            },
            server_info: session::ImplementationInfo {
                name: "weather-example".to_string(),
                version: "0.1.0".to_string(),
                title: Some("Weather Example Server".to_string()),
            },
            instructions: None,
            meta: None,
        })
    }

    fn handle_initialized() -> Result<(), types::McpError> {
        Ok(())
    }

    fn handle_ping() -> Result<(), types::McpError> {
        Ok(())
    }

    fn handle_shutdown() -> Result<(), types::McpError> {
        Ok(())
    }
}

// Implement tool handlers (our main functionality)
impl ToolGuest for Component {
    fn handle_list_tools(_request: tools::ListToolsRequest) -> Result<tools::ListToolsResponse, types::McpError> {
        Ok(tools::ListToolsResponse {
            tools: vec![
                tools::Tool {
                    base: types::BaseMetadata {
                        name: "echo".to_string(),
                        title: Some("Echo Tool".to_string()),
                    },
                    description: Some("Echo a message back to the user".to_string()),
                    input_schema: r#"{"type":"object","properties":{"message":{"type":"string"}},"required":["message"]}"#.to_string(),
                    output_schema: None,
                    annotations: None,
                    meta: None,
                },
                tools::Tool {
                    base: types::BaseMetadata {
                        name: "weather".to_string(),
                        title: Some("Weather Tool".to_string()),
                    },
                    description: Some("Get weather information for a location".to_string()),
                    input_schema: r#"{"type":"object","properties":{"location":{"type":"string"}},"required":["location"]}"#.to_string(),
                    output_schema: None,
                    annotations: None,
                    meta: None,
                },
            ],
            next_cursor: None,
            meta: None,
        })
    }

    fn handle_call_tool(request: tools::CallToolRequest) -> Result<tools::ToolResult, types::McpError> {
        let args = if let Some(args_str) = &request.arguments {
            serde_json::from_str(args_str)
                .map_err(|e| types::McpError {
                    code: types::ErrorCode::InvalidParams,
                    message: format!("Invalid arguments: {}", e),
                    data: None,
                })?
        } else {
            serde_json::Value::Object(serde_json::Map::new())
        };

        match request.name.as_str() {
            "echo" => {
                let message = args["message"].as_str()
                    .ok_or_else(|| types::McpError {
                        code: types::ErrorCode::InvalidParams,
                        message: "Missing message field".to_string(),
                        data: None,
                    })?;

                Ok(tools::ToolResult {
                    content: vec![types::ContentBlock::Text(types::TextContent {
                        text: format!("Echo: {}", message),
                        annotations: None,
                        meta: None,
                    })],
                    is_error: Some(false),
                    structured_content: None,
                    meta: None,
                })
            }
            "weather" => {
                let location = args["location"].as_str()
                    .ok_or_else(|| types::McpError {
                        code: types::ErrorCode::InvalidParams,
                        message: "Missing location field".to_string(),
                        data: None,
                    })?;

                let weather_result = spin_executor::run(get_weather(location.to_string()));
                
                match weather_result {
                    Ok(text) => Ok(tools::ToolResult {
                        content: vec![types::ContentBlock::Text(types::TextContent {
                            text,
                            annotations: None,
                            meta: None,
                        })],
                        is_error: Some(false),
                        structured_content: None,
                        meta: None,
                    }),
                    Err(e) => Ok(tools::ToolResult {
                        content: vec![types::ContentBlock::Text(types::TextContent {
                            text: format!("Error: {}", e),
                            annotations: None,
                            meta: None,
                        })],
                        is_error: Some(true),
                        structured_content: None,
                        meta: None,
                    })
                }
            }
            _ => Err(types::McpError {
                code: types::ErrorCode::ToolNotFound,
                message: format!("Unknown tool: {}", request.name),
                data: None,
            })
        }
    }
}

// Async weather fetching with Spin SDK
async fn get_weather(location: String) -> Result<String, String> {
    use spin_sdk::http::{Request, send};
    
    #[derive(Deserialize)]
    struct GeocodingResponse {
        results: Option<Vec<Location>>,
    }
    
    #[derive(Deserialize)]
    struct Location {
        latitude: f64,
        longitude: f64,
        name: String,
        country: String,
    }
    
    #[derive(Deserialize)]
    struct WeatherResponse {
        current_weather: CurrentWeather,
    }
    
    #[derive(Deserialize)]
    struct CurrentWeather {
        temperature: f64,
        windspeed: f64,
        weathercode: i32,
    }
    
    // Get coordinates for the location
    let geocoding_url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&format=json",
        url::form_urlencoded::byte_serialize(location.as_bytes()).collect::<String>()
    );
    
    let geocoding_request = Request::get(&geocoding_url);
    let geocoding_response: spin_sdk::http::Response = send(geocoding_request).await
        .map_err(|e| format!("Failed to fetch location data: {:?}", e))?;
    
    let geocoding_body = geocoding_response.body().to_vec();
    let geocoding: GeocodingResponse = serde_json::from_slice(&geocoding_body)
        .map_err(|e| format!("Failed to parse location data: {}", e))?;
    
    let location_data = geocoding.results
        .and_then(|r| r.into_iter().next())
        .ok_or_else(|| format!("Location '{}' not found", location))?;
    
    // Get weather for the coordinates  
    let weather_url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current_weather=true",
        location_data.latitude,
        location_data.longitude
    );
    
    let weather_request = Request::get(&weather_url);
    let weather_response: spin_sdk::http::Response = send(weather_request).await
        .map_err(|e| format!("Failed to fetch weather data: {:?}", e))?;
    
    let weather_body = weather_response.body().to_vec();
    let weather: WeatherResponse = serde_json::from_slice(&weather_body)
        .map_err(|e| format!("Failed to parse weather data: {}", e))?;
    
    let weather_description = match weather.current_weather.weathercode {
        0 => "Clear sky",
        1..=3 => "Partly cloudy",
        45 | 48 => "Foggy",
        51..=57 => "Drizzle",
        61..=67 => "Rain",
        71..=77 => "Snow",
        80..=82 => "Rain showers",
        85 | 86 => "Snow showers",
        95 | 96 | 99 => "Thunderstorm",
        _ => "Unknown",
    };
    
    Ok(format!(
        "Weather for {}, {}:\n🌡️ Temperature: {:.1}°C\n☁️ Conditions: {}\n💨 Wind: {:.1} km/h",
        location_data.name,
        location_data.country,
        weather.current_weather.temperature,
        weather_description,
        weather.current_weather.windspeed
    ))
}

// Stub implementations for resource handler (we don't provide resources)
impl ResourceGuest for Component {
    fn handle_list_resources(_request: resources::ListResourcesRequest) -> Result<resources::ListResourcesResponse, types::McpError> {
        Ok(resources::ListResourcesResponse {
            resources: vec![],
            next_cursor: None,
            meta: None,
        })
    }

    fn handle_list_resource_templates(_request: resources::ListTemplatesRequest) -> Result<resources::ListTemplatesResponse, types::McpError> {
        Ok(resources::ListTemplatesResponse {
            templates: vec![],
            next_cursor: None,
            meta: None,
        })
    }

    fn handle_read_resource(_request: resources::ReadResourceRequest) -> Result<resources::ReadResourceResponse, types::McpError> {
        Err(types::McpError {
            code: types::ErrorCode::ResourceNotFound,
            message: "This server does not provide resources".to_string(),
            data: None,
        })
    }

    fn handle_subscribe_resource(_request: resources::SubscribeRequest) -> Result<(), types::McpError> {
        Ok(())
    }

    fn handle_unsubscribe_resource(_request: resources::UnsubscribeRequest) -> Result<(), types::McpError> {
        Ok(())
    }
}

// Stub implementations for prompt handler (we don't provide prompts)
impl PromptGuest for Component {
    fn handle_list_prompts(_request: prompts::ListPromptsRequest) -> Result<prompts::ListPromptsResponse, types::McpError> {
        Ok(prompts::ListPromptsResponse {
            prompts: vec![],
            next_cursor: None,
            meta: None,
        })
    }

    fn handle_get_prompt(_request: prompts::GetPromptRequest) -> Result<prompts::GetPromptResponse, types::McpError> {
        Err(types::McpError {
            code: types::ErrorCode::PromptNotFound,
            message: "This server does not provide prompts".to_string(),
            data: None,
        })
    }
}

bindings::export!(Component with_types_in bindings);