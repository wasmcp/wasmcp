use wasmcp::{AsyncToolHandler, AsyncResourceHandler, json};
use spin_sdk::http::{Method, Request, Response, send};
use spin_sdk::key_value::Store;
use serde::Deserialize;

// Define your tools as zero-sized types
struct EchoTool;

impl AsyncToolHandler for EchoTool {
    const NAME: &'static str = "echo";
    const DESCRIPTION: &'static str = "Echo a message back to the user";
    
    fn input_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "message": { 
                    "type": "string", 
                    "description": "Message to echo back" 
                }
            },
            "required": ["message"]
        })
    }
    
    async fn execute_async(args: serde_json::Value) -> Result<String, String> {
        let message = args["message"]
            .as_str()
            .ok_or("Missing message field")?;
        
        Ok(format!("Echo: {}", message))
    }
}

// Weather tool using real Open-Meteo API
struct WeatherTool;

#[derive(Deserialize)]
struct GeocodingResponse {
    results: Option<Vec<GeocodingResult>>,
}

#[derive(Deserialize)]
struct GeocodingResult {
    latitude: f64,
    longitude: f64,
    name: String,
}

#[derive(Deserialize)]
struct WeatherResponse {
    current: CurrentWeather,
}

#[derive(Deserialize)]
struct CurrentWeather {
    temperature_2m: f64,
    apparent_temperature: f64,
    relative_humidity_2m: f64,
    wind_speed_10m: f64,
    weather_code: i32,
}

fn get_weather_condition(code: i32) -> &'static str {
    match code {
        0 => "Clear sky",
        1..=3 => "Partly cloudy",
        45 | 48 => "Foggy",
        51..=57 => "Drizzle",
        61..=67 => "Rain",
        71..=77 => "Snow",
        80..=82 => "Rain showers",
        85 | 86 => "Snow showers",
        95..=99 => "Thunderstorm",
        _ => "Unknown",
    }
}

impl AsyncToolHandler for WeatherTool {
    const NAME: &'static str = "weather";
    const DESCRIPTION: &'static str = "Get current weather for any city using Open-Meteo API";
    
    fn input_schema() -> serde_json::Value {
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
    }
    
    async fn execute_async(args: serde_json::Value) -> Result<String, String> {
        let location = args["location"]
            .as_str()
            .ok_or("Missing location field")?;
        
        // First, geocode the location
        let geocoding_url = format!(
            "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1",
            urlencoding::encode(location)
        );
        
        let geocoding_request = Request::builder()
            .method(Method::Get)
            .uri(geocoding_url)
            .build();
        
        let geocoding_response: Response = send(geocoding_request).await
            .map_err(|e| format!("Failed to fetch geocoding data: {}", e))?;
        
        if *geocoding_response.status() != 200 {
            return Err(format!("Geocoding API returned status: {}", geocoding_response.status()));
        }
        
        let geocoding_data: GeocodingResponse = serde_json::from_slice(geocoding_response.body())
            .map_err(|e| format!("Failed to parse geocoding response: {}", e))?;
        
        let geocoding_result = geocoding_data.results
            .and_then(|r| r.into_iter().next())
            .ok_or_else(|| format!("Location '{}' not found", location))?;
        
        // Now fetch the weather data
        let weather_url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code",
            geocoding_result.latitude,
            geocoding_result.longitude
        );
        
        let weather_request = Request::builder()
            .method(Method::Get)
            .uri(weather_url)
            .build();
        
        let weather_response: Response = send(weather_request).await
            .map_err(|e| format!("Failed to fetch weather data: {}", e))?;
        
        if *weather_response.status() != 200 {
            return Err(format!("Weather API returned status: {}", weather_response.status()));
        }
        
        let weather_data: WeatherResponse = serde_json::from_slice(weather_response.body())
            .map_err(|e| format!("Failed to parse weather response: {}", e))?;
        
        let current = &weather_data.current;
        let conditions = get_weather_condition(current.weather_code);
        
        Ok(format!(
            "Weather in {}:\n\
            Temperature: {:.1}°C (feels like {:.1}°C)\n\
            Conditions: {}\n\
            Humidity: {:.0}%\n\
            Wind: {:.1} km/h",
            geocoding_result.name,
            current.temperature_2m,
            current.apparent_temperature,
            conditions,
            current.relative_humidity_2m,
            current.wind_speed_10m
        ))
    }
}

// Simple URL encoding helper
mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
                ' ' => "+".to_string(),
                _ => format!("%{:02X}", c as u8),
            })
            .collect()
    }
}

// Example of key-value storage tool using Spin SDK
struct KeyValueTool;

impl AsyncToolHandler for KeyValueTool {
    const NAME: &'static str = "key_value";
    const DESCRIPTION: &'static str = "Store and retrieve data using Spin's key-value store";
    
    fn input_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["get", "set", "delete", "exists", "list"],
                    "description": "Operation to perform"
                },
                "key": {
                    "type": "string",
                    "description": "Key to operate on"
                },
                "value": {
                    "type": "string",
                    "description": "Value to store (for set operation)"
                },
                "store": {
                    "type": "string",
                    "description": "Name of the key-value store to use",
                    "default": "default"
                }
            },
            "required": ["operation"]
        })
    }
    
    async fn execute_async(args: serde_json::Value) -> Result<String, String> {
        let operation = args["operation"]
            .as_str()
            .ok_or("Missing operation field")?;
        
        let store_name = args["store"]
            .as_str()
            .unwrap_or("default");
        
        let store = Store::open(store_name)
            .map_err(|e| format!("Failed to open store '{}': {}", store_name, e))?;
        
        match operation {
            "get" => {
                let key = args["key"]
                    .as_str()
                    .ok_or("Missing key field")?;
                
                match store.get(key) {
                    Ok(Some(value)) => {
                        let value_str = String::from_utf8_lossy(&value);
                        Ok(json!({
                            "found": true,
                            "value": value_str
                        }).to_string())
                    },
                    Ok(None) => Ok(json!({
                        "found": false,
                        "message": format!("Key '{}' not found", key)
                    }).to_string()),
                    Err(e) => Err(format!("Failed to get key '{}': {}", key, e))
                }
            },
            "set" => {
                let key = args["key"]
                    .as_str()
                    .ok_or("Missing key field")?;
                let value = args["value"]
                    .as_str()
                    .ok_or("Missing value field")?;
                
                store.set(key, value.as_bytes())
                    .map_err(|e| format!("Failed to set key '{}': {}", key, e))?;
                
                Ok(json!({
                    "success": true,
                    "message": format!("Set key '{}' to value", key)
                }).to_string())
            },
            "delete" => {
                let key = args["key"]
                    .as_str()
                    .ok_or("Missing key field")?;
                
                store.delete(key)
                    .map_err(|e| format!("Failed to delete key '{}': {}", key, e))?;
                
                Ok(json!({
                    "success": true,
                    "message": format!("Deleted key '{}'", key)
                }).to_string())
            },
            "exists" => {
                let key = args["key"]
                    .as_str()
                    .ok_or("Missing key field")?;
                
                match store.exists(key) {
                    Ok(exists) => Ok(json!({
                        "exists": exists,
                        "key": key
                    }).to_string()),
                    Err(e) => Err(format!("Failed to check key '{}': {}", key, e))
                }
            },
            "list" => {
                match store.get_keys() {
                    Ok(keys) => Ok(json!({
                        "keys": keys,
                        "count": keys.len()
                    }).to_string()),
                    Err(e) => Err(format!("Failed to list keys: {}", e))
                }
            },
            _ => Err(format!("Unknown operation: {}", operation))
        }
    }
}

// Example resource that provides system information
struct SystemInfoResource;

impl AsyncResourceHandler for SystemInfoResource {
    const URI: &'static str = "system://info";
    const NAME: &'static str = "System Information";
    const DESCRIPTION: Option<&'static str> = Some("Provides information about the MCP server");
    const MIME_TYPE: Option<&'static str> = Some("application/json");
    
    async fn read_async() -> Result<String, String> {
        Ok(json!({
            "version": "0.1.0",
            "capabilities": [
                "weather_api",
                "key_value_storage",
                "http_outbound"
            ],
            "supported_apis": [
                "Open-Meteo Weather API",
                "Open-Meteo Geocoding API"
            ]
        }).to_string())
    }
}

// Generate the MCP handler implementation
// This macro generates WebAssembly bindings, so it's only compiled for wasm targets
#[cfg(target_arch = "wasm32")]
wasmcp::create_handler!(
    tools: [EchoTool, WeatherTool, KeyValueTool],
    resources: [SystemInfoResource],
);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[tokio::test]
    async fn test_echo_tool() {
        let args = json!({
            "message": "Hello, MCP!"
        });
        
        let result = EchoTool::execute_async(args).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Echo: Hello, MCP!");
    }
    
    #[tokio::test]
    async fn test_weather_tool_schema() {
        let schema = WeatherTool::input_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["location"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&json!("location")));
    }
}