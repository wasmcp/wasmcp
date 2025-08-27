"""Example echo and weather handler for wasmcp."""

from wasmcp import Handler
import json
from urllib.parse import quote
from typing import Optional

# Import Spin SDK capabilities
from spin_sdk import http
from spin_sdk.http import Request, Response
from spin_sdk.key_value import Store

### Pay attention to correcting this
# from mcp_sdk import api as mcp

# Create handler instance
handler = Handler("echo-weather-handler")


def make_http_request(url: str) -> Optional[str]:
    """Make a simple HTTP GET request using Spin SDK."""
    try:
        # Use Spin SDK's http.send to make the request
        req = Request("GET", url, {}, None)
        resp = http.send(req)
        
        # Return the response body as a string
        if resp.status == 200:
            return resp.body.decode('utf-8') if resp.body else None
        else:
            return json.dumps({
                "error": f"HTTP request failed with status {resp.status}",
                "url": url
            })
    except Exception as e:
        return json.dumps({
            "error": f"HTTP request failed: {str(e)}",
            "url": url
        })


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
    
    Args:
        location: The location to get weather for
        
    Returns:
        Weather information as JSON string
    """
    # URL-encode the location to handle spaces and special characters
    encoded_location = quote(location)
    
    # Make a request to wttr.in API for weather data
    url = f"https://wttr.in/{encoded_location}?format=j1"
    
    try:
        response = make_http_request(url)
        if response:
            # Parse the JSON response
            weather_data = json.loads(response)
            
            # Extract relevant information
            current = weather_data.get("current_condition", [{}])[0]
            
            result = {
                "location": location,
                "temperature": f"{current.get('temp_C', 'N/A')}°C ({current.get('temp_F', 'N/A')}°F)",
                "feels_like": f"{current.get('FeelsLikeC', 'N/A')}°C ({current.get('FeelsLikeF', 'N/A')}°F)",
                "description": current.get("weatherDesc", [{}])[0].get("value", "N/A"),
                "humidity": f"{current.get('humidity', 'N/A')}%",
                "wind_speed": f"{current.get('windspeedKmph', 'N/A')} km/h",
                "wind_direction": current.get("winddir16Point", "N/A"),
                "pressure": f"{current.get('pressure', 'N/A')} mb",
                "visibility": f"{current.get('visibility', 'N/A')} km",
                "uv_index": current.get("uvIndex", "N/A"),
                "observation_time": current.get("observation_time", "N/A")
            }
            
            return json.dumps(result, indent=2)
        else:
            return json.dumps({
                "error": "Failed to fetch weather data",
                "location": location
            }, indent=2)
    except json.JSONDecodeError as e:
        return json.dumps({
            "error": f"Failed to parse weather data: {str(e)}",
            "location": location
        }, indent=2)
    except Exception as e:
        return json.dumps({
            "error": f"Error fetching weather: {str(e)}",
            "location": location
        }, indent=2)


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


@handler.tool(description="Store a value in Spin's key-value store")
def kv_set(key: str, value: str) -> str:
    """Store a value in Spin's key-value store.
    
    Args:
        key: The key to store
        value: The value to store
        
    Returns:
        Success message or error
    """
    try:
        store = Store.open("default")
        store.set(key, value.encode('utf-8'))
        return f"Successfully stored value for key: {key}"
    except Exception as e:
        return f"Failed to store value: {str(e)}"


@handler.tool(description="Retrieve a value from Spin's key-value store")
def kv_get(key: str) -> str:
    """Retrieve a value from Spin's key-value store.
    
    Args:
        key: The key to retrieve
        
    Returns:
        The stored value or error message
    """
    try:
        store = Store.open("default")
        value = store.get(key)
        if value:
            return value.decode('utf-8')
        else:
            return f"Key not found: {key}"
    except Exception as e:
        return f"Failed to retrieve value: {str(e)}"


# The handler instance is used by exports.py which implements the WIT interface
__all__ = ['list_tools', 'call_tool', 'list_resources', 'read_resource', 'list_prompts', 'get_prompt']