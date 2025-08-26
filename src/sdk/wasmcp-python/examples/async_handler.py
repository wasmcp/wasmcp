"""Example async MCP handler using Spin SDK for HTTP requests.

This demonstrates how to build async tools that can make HTTP requests,
similar to the Rust SDK's async support.
"""

from wasmcp import WasmcpHandler
from typing import Dict, Any
import json

# Create handler
handler = WasmcpHandler("async-weather-example")

@handler.tool(name="weather", description="Get weather for a location")
async def get_weather(location: str) -> Dict[str, Any]:
    """Fetch weather data for a location using async HTTP.
    
    This uses Spin's HTTP client for async requests in WASI environment.
    """
    # Import Spin SDK for HTTP requests (available in WASI environment)
    try:
        from spin_sdk import http
        
        # Make async HTTP request
        response = await http.send(
            http.Request(
                "GET",
                f"https://api.weather.example.com/v1/weather?q={location}",
                headers={"Accept": "application/json"}
            )
        )
        
        if response.status == 200:
            data = json.loads(response.body)
            return {
                "location": location,
                "temperature": data.get("temp"),
                "condition": data.get("condition"),
                "humidity": data.get("humidity")
            }
        else:
            return {
                "error": f"Weather API returned status {response.status}"
            }
            
    except ImportError:
        # Fallback for local testing (not in WASI environment)
        return {
            "location": location,
            "temperature": 72,
            "condition": "sunny",
            "humidity": 45,
            "_mock": True
        }

@handler.tool(name="forecast", description="Get weather forecast")
async def get_forecast(location: str, days: int = 5) -> Dict[str, Any]:
    """Get multi-day weather forecast.
    
    Another async tool demonstrating parameter handling.
    """
    try:
        from spin_sdk import http
        
        response = await http.send(
            http.Request(
                "GET",
                f"https://api.weather.example.com/v1/forecast?q={location}&days={days}",
                headers={"Accept": "application/json"}
            )
        )
        
        if response.status == 200:
            data = json.loads(response.body)
            return {
                "location": location,
                "days": days,
                "forecast": data.get("daily", [])
            }
        else:
            return {
                "error": f"Forecast API returned status {response.status}"
            }
            
    except ImportError:
        # Mock data for testing
        return {
            "location": location,
            "days": days,
            "forecast": [
                {"day": i, "high": 75 + i, "low": 60 + i}
                for i in range(days)
            ],
            "_mock": True
        }

# Sync tool for comparison
@handler.tool(name="echo", description="Echo a message")
def echo_message(message: str) -> str:
    """Simple sync tool that echoes a message."""
    return f"Echo: {message}"

# Resource that could use async internally
@handler.resource(uri="weather://current/all", mime_type="application/json")
async def get_all_current_weather() -> dict:
    """Get current weather for all major cities.
    
    This resource could aggregate data from multiple async calls.
    """
    cities = ["New York", "London", "Tokyo", "Sydney"]
    
    # In a real implementation, these could be parallel async calls
    weather_data = {}
    for city in cities:
        # Reuse the async tool
        weather_data[city] = await get_weather(city)
    
    return {
        "cities": weather_data,
        "timestamp": "2024-01-01T00:00:00Z"  # Would use real timestamp
    }

# Build the handler for WASM compilation
handler.build()

if __name__ == "__main__":
    # For local testing
    import asyncio
    from wasmcp.exports import WasmcpExports
    
    async def test():
        exports = WasmcpExports(handler)
        
        # Test async tool
        result = exports.call_tool("weather", json.dumps({"location": "Seattle"}))
        print(f"Weather result: {result}")
        
        # Test sync tool
        result = exports.call_tool("echo", json.dumps({"message": "Hello async!"}))
        print(f"Echo result: {result}")
    
    # Run test
    asyncio.run(test())