"""
Python MCP Weather Server

A WebAssembly component that provides weather tools using a decorator-based API.
Implements three example tools:
- echo: Echo a message back
- get_weather: Get weather for a single location  
- multi_weather: Get weather for multiple locations concurrently
"""

import json
import asyncio
import urllib.parse
from typing import List, Dict, Any

from helpers import MCPServer, fetch_json, json_result

# Create the MCP server
mcp = MCPServer(
    name="Weather Server",
    instructions="A weather information provider with echo capabilities"
)


@mcp.tool
def echo(message: str) -> str:
    """Echo a message back to the user."""
    return f"Echo: {message}"


@mcp.tool("get_weather")
async def get_weather_tool(location: str) -> str:
    """Get current weather for a location."""
    weather_data = await fetch_weather(location)
    formatted = format_weather(weather_data)
    return json.dumps(formatted, indent=2)


@mcp.tool(name="multi_weather", description="Get weather for multiple cities concurrently")
async def get_multi_weather(cities: List[str]) -> str:
    """Fetch weather for multiple cities in parallel."""
    # Execute concurrent weather fetches
    tasks = [fetch_weather(city) for city in cities]
    results = await asyncio.gather(*tasks, return_exceptions=True)
    
    # Format results
    formatted_results = []
    for city, result in zip(cities, results):
        if isinstance(result, Exception):
            formatted_results.append({
                "city": city,
                "error": str(result)
            })
        else:
            formatted_results.append(format_weather(result))
    
    return json.dumps(formatted_results, indent=2)


# Helper functions for weather fetching

async def fetch_weather(city: str) -> Dict[str, Any]:
    """Fetch weather data for a single city."""
    # Geocode the location
    geo_url = f"https://geocoding-api.open-meteo.com/v1/search?name={urllib.parse.quote(city)}&count=1"
    geo_data = await fetch_json(geo_url)
    
    if not geo_data.get("results"):
        raise Exception(f"Location '{city}' not found")
    
    location = geo_data["results"][0]
    
    # Get weather data
    weather_url = (
        f"https://api.open-meteo.com/v1/forecast?"
        f"latitude={location['latitude']}&longitude={location['longitude']}"
        f"&current=temperature_2m,relative_humidity_2m,wind_speed_10m,weather_code"
    )
    weather = await fetch_json(weather_url)
    
    return {
        "name": location["name"],
        "country": location["country"],
        "latitude": location["latitude"],
        "longitude": location["longitude"],
        "temperature": weather["current"]["temperature_2m"],
        "humidity": weather["current"]["relative_humidity_2m"],
        "wind_speed": weather["current"]["wind_speed_10m"],
        "weather_code": weather["current"]["weather_code"]
    }


def format_weather(data: Dict[str, Any]) -> Dict[str, Any]:
    """Format weather data for display."""
    return {
        "location": f"{data['name']}, {data['country']}",
        "temperature": f"{data['temperature']:.1f}°C",
        "conditions": get_weather_condition(data['weather_code']),
        "humidity": f"{data['humidity']}%",
        "wind": f"{data['wind_speed']:.1f} m/s"
    }


def get_weather_condition(code: int) -> str:
    """Convert weather code to human-readable condition."""
    conditions = {
        0: "Clear sky",
        1: "Mainly clear",
        2: "Partly cloudy",
        3: "Overcast",
        45: "Foggy",
        48: "Depositing rime fog",
        51: "Light drizzle",
        53: "Moderate drizzle",
        55: "Dense drizzle",
        61: "Slight rain",
        63: "Moderate rain",
        65: "Heavy rain",
        71: "Slight snow fall",
        73: "Moderate snow fall",
        75: "Heavy snow fall",
        80: "Slight rain showers",
        81: "Moderate rain showers",
        82: "Violent rain showers",
        95: "Thunderstorm",
        96: "Thunderstorm with slight hail",
        99: "Thunderstorm with heavy hail"
    }
    return conditions.get(code, f"Weather code {code}")


# Export the capabilities handler class for the WebAssembly component
# componentize-py expects a class, not an instance
ToolsCapabilities = mcp.get_capabilities_class()