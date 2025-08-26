use serde::Deserialize;
use serde_json::json;

// Simple echo tool for testing
struct EchoTool;

impl wasmcp::ToolHandler for EchoTool {
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
    
    fn execute(args: serde_json::Value) -> Result<String, String> {
        let message = args["message"]
            .as_str()
            .ok_or("Missing message field")?;
        
        Ok(format!("Echo: {}", message))
    }
}

// Weather tool demonstrating real async HTTP requests
struct WeatherTool;

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

impl wasmcp::AsyncToolHandler for WeatherTool {
    const NAME: &'static str = "weather";
    const DESCRIPTION: &'static str = "Get weather information for a location";
    
    fn input_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name or location"
                }
            },
            "required": ["location"]
        })
    }
    
    async fn execute_async(args: serde_json::Value) -> Result<String, String> {
        let location = args["location"]
            .as_str()
            .ok_or("Missing location field")?;
        
        // Use spin_sdk for HTTP requests (works with any WASI HTTP runtime)
        use spin_sdk::http::{Request, send};
        
        // Step 1: Get coordinates for the location
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
        
        // Step 2: Get weather for the coordinates
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
        
        // Format the weather response
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
}

// Generate the MCP handler implementation
#[cfg(target_arch = "wasm32")]
wasmcp::create_handler!(
    tools: [EchoTool, WeatherTool],
);