"""Example echo and weather handler for wasmcp."""

from wasmcp import Handler
import json

# Create handler instance
handler = Handler("echo-weather-handler")


@handler.tool
def echo(message: str) -> str:
    """Echo back the provided message.
    
    Args:
        message: The message to echo back
        
    Returns:
        The echoed message
    """
    return f"Echo: {message}"


@handler.tool
def reverse(text: str) -> str:
    """Reverse the provided text.
    
    Args:
        text: The text to reverse
        
    Returns:
        The reversed text
    """
    return text[::-1]


@handler.tool(name="shout", description="Convert text to uppercase")
def make_uppercase(text: str) -> str:
    """Convert text to uppercase.
    
    Args:
        text: The text to convert
        
    Returns:
        The uppercase text
    """
    return text.upper()


@handler.resource(uri="config://version")
def get_version() -> dict:
    """Get the handler version information."""
    return {
        "name": "echo-handler",
        "version": "1.0.0",
        "sdk": "wasmcp-python"
    }


@handler.resource(
    uri="data://capabilities",
    mime_type="application/json",
    description="Handler capabilities"
)
def get_capabilities() -> dict:
    """Get handler capabilities."""
    return {
        "tools": ["echo", "reverse", "shout"],
        "resources": ["config://version", "data://capabilities"],
        "features": ["text-manipulation", "configuration"]
    }


@handler.tool(description="Get current weather for a location")
def weather(location: str) -> str:
    """Get current weather for a location.
    
    Note: This is a mock implementation for the example.
    In a real implementation, you would call an actual weather API.
    
    Args:
        location: The location to get weather for
        
    Returns:
        Weather information as JSON string
    """
    # Mock weather data - in real implementation, call weather API
    mock_weather = {
        "location": location,
        "temperature": "22°C",
        "condition": "Partly cloudy",
        "humidity": "65%",
        "wind": "10 km/h NW",
        "timestamp": "2025-01-26T10:00:00Z"
    }
    return json.dumps(mock_weather, indent=2)


@handler.prompt
def greeting_prompt(name: str = "World") -> list:
    """Generate a greeting prompt.
    
    Args:
        name: Name to greet (default: World)
        
    Returns:
        List of prompt messages
    """
    return [
        {"role": "system", "content": "You are a friendly assistant."},
        {"role": "user", "content": f"Please greet {name} warmly."}
    ]


@handler.prompt(description="Generate a weather analysis prompt")
def weather_analysis_prompt(location: str = "your area") -> list:
    """Generate a prompt for weather analysis.
    
    Args:
        location: Location for weather analysis
        
    Returns:
        List of prompt messages for weather analysis
    """
    return [
        {"role": "system", "content": "You are a meteorologist providing weather analysis."},
        {"role": "user", "content": f"Analyze the weather patterns for {location} and provide insights."}
    ]


# Set up WIT exports by importing them - this makes them available to componentize-py
from wasmcp.wit.exports import (
    list_tools, call_tool, list_resources, 
    read_resource, list_prompts, get_prompt, set_handler
)

# Register this handler for the exports
set_handler(handler)

# Export the functions at module level for componentize-py to find
__all__ = ['list_tools', 'call_tool', 'list_resources', 'read_resource', 'list_prompts', 'get_prompt']