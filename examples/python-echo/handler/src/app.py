"""Example echo and weather handler for wasmcp."""

from wasmcp import Handler
import json
from urllib.parse import urlparse, quote
from wit_world.imports import wasi_http_types, outgoing_handler, poll
from wit_world.types import Ok
from typing import Optional

# Create handler instance
handler = Handler("echo-weather-handler")


def make_http_request(url: str) -> Optional[str]:
    """Make a simple HTTP GET request using WASI HTTP bindings."""
    # Parse the URL
    parsed = urlparse(url)
    
    # Create the request headers
    headers = wasi_http_types.Fields.from_list([])
    
    # Create the outgoing request
    request = wasi_http_types.OutgoingRequest(headers)
    
    # Set the scheme
    if parsed.scheme == "https":
        request.set_scheme(wasi_http_types.Scheme_Https())
    else:
        request.set_scheme(wasi_http_types.Scheme_Http())
    
    # Set the authority (host:port)
    request.set_authority(parsed.netloc)
    
    # Set the path with query
    path = parsed.path if parsed.path else "/"
    if parsed.query:
        path += "?" + parsed.query
    request.set_path_with_query(path)
    
    # Send the request
    try:
        future_response = outgoing_handler.handle(request, None)
        
        # Poll until response is ready
        pollable = future_response.subscribe()
        poll.poll([pollable])
        
        # Get the response
        response_result = future_response.get()
        if response_result is None:
            return None
            
        # Extract the actual response
        if isinstance(response_result, Ok):
            inner_result = response_result.value
            if isinstance(inner_result, Ok):
                response = inner_result.value
                
                # Read the response body
                body = response.consume()
                stream = body.stream()
                
                # Read chunks
                data = bytearray()
                while True:
                    try:
                        chunk = stream.blocking_read(8192)
                        if isinstance(chunk, bytes):
                            if len(chunk) == 0:
                                break
                            data.extend(chunk)
                        else:
                            # Handle other possible return types
                            break
                    except Exception as e:
                        # StreamError_Closed is expected when stream ends
                        if "Closed" in str(e):
                            break
                        raise
                
                return data.decode('utf-8', errors='replace')
    except Exception as e:
        return f"Error: {str(e)}"
    
    return None


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


# The handler instance is used by exports.py which implements the WIT interface
__all__ = ['list_tools', 'call_tool', 'list_resources', 'read_resource', 'list_prompts', 'get_prompt']