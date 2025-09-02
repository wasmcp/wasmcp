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
from typing import List, Dict, Any, Optional

from helpers import MCPServer, fetch_json, text_result
from wit_world.imports.authorization_types import ProviderAuthConfig

# ==============================================================================
# MCP SERVER CONFIGURATION
# ==============================================================================

# Create the MCP server
mcp = MCPServer(
    name="weather-py",
    version="0.1.0",
    instructions="Weather Python MCP Server",
    auth_config=None,
    # auth_config=ProviderAuthConfig(
    #     expected_issuer="https://xxxxx.authkit.app",
    #     expected_audiences=["client_xxxxx"],
    #     jwks_uri="https://xxxxx.authkit.app/oauth2/jwks",
    #     policy=None,  # Optional: Add Rego policy for additional authorization
    #     policy_data=None,  # Optional: Add policy data as JSON string
    # )
)


@mcp.tool
def echo(message: str) -> str:
    """Echo a message back to the user."""
    return f"Echo: {message}"


@mcp.tool("get_weather")
async def get_weather_tool(location: str) -> str:
    """Get current weather for a location."""
    try:
        weather_data = await fetch_weather(location)
        return format_weather_text(weather_data)
    except Exception as e:
        return f"Error fetching weather: {e}"


@mcp.tool(name="multi_weather", description="Get weather for multiple cities concurrently")
async def get_multi_weather(cities: List[str]) -> str:
    """Fetch weather for multiple cities in parallel."""
    if not cities:
        return "No cities provided"
    
    if len(cities) > 5:
        return "Maximum 5 cities allowed"
    
    # Execute concurrent weather fetches
    tasks = [fetch_weather(city) for city in cities]
    results = await asyncio.gather(*tasks, return_exceptions=True)
    
    # Format results
    output = "=== Weather Results ===\n\n"
    for city, result in zip(cities, results):
        if isinstance(result, Exception):
            output += f"Error fetching weather for {city}: {result}\n\n"
        else:
            output += format_weather_text(result) + "\n\n"
    output += "=== All requests completed ==="
    
    return output


# Helper functions for weather fetching

async def fetch_weather(city: str) -> Dict[str, Any]:
    """Fetch weather data for a single city."""
    # Geocode the location
    geo_url = f"https://geocoding-api.open-meteo.com/v1/search?name={urllib.parse.quote(city)}&count=1"
    geo_data = await fetch_json(geo_url)
    
    if not geo_data.get("results"):
        raise Exception(f"Location '{city}' not found")
    
    location = geo_data["results"][0]
    
    # Get weather data with apparent temperature
    weather_url = (
        f"https://api.open-meteo.com/v1/forecast?"
        f"latitude={location['latitude']}&longitude={location['longitude']}"
        f"&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code"
    )
    weather = await fetch_json(weather_url)
    
    return {
        "name": location["name"],
        "country": location["country"],
        "latitude": location["latitude"],
        "longitude": location["longitude"],
        "temperature": weather["current"]["temperature_2m"],
        "apparent_temperature": weather["current"]["apparent_temperature"],
        "humidity": weather["current"]["relative_humidity_2m"],
        "wind_speed": weather["current"]["wind_speed_10m"],
        "weather_code": weather["current"]["weather_code"]
    }


def format_weather_text(data: Dict[str, Any]) -> str:
    """Format weather data as text matching weather-rs output."""
    return (
        f"Weather in {data['name']}, {data['country']}:\n"
        f"Temperature: {data['temperature']:.1f}°C (feels like {data['apparent_temperature']:.1f}°C)\n"
        f"Conditions: {get_weather_condition(data['weather_code'])}\n"
        f"Humidity: {data['humidity']}%\n"
        f"Wind: {data['wind_speed']:.1f} km/h"
    )


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
        56: "Light freezing drizzle",
        57: "Dense freezing drizzle",
        66: "Light freezing rain",
        67: "Heavy freezing rain",
        77: "Snow grains",
        80: "Slight rain showers",
        81: "Moderate rain showers",
        82: "Violent rain showers",
        85: "Slight snow showers",
        86: "Heavy snow showers",
        95: "Thunderstorm",
        96: "Thunderstorm with slight hail",
        99: "Thunderstorm with heavy hail"
    }
    return conditions.get(code, "Unknown")


# Export the capabilities handler classes for the WebAssembly component
# componentize-py expects classes, not instances
capabilities_class = mcp.get_capabilities_class()
ToolsCapabilities = capabilities_class
CoreCapabilities = capabilities_class