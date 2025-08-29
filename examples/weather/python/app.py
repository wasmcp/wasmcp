"""
Python MCP Weather Handler

Demonstrates the clean helper API for building MCP tools in Python.
Implements the same tools as the Rust and JavaScript examples:
- echo: Echo a message back
- get_weather: Get weather for a single location
- multi_weather: Get weather for multiple locations concurrently
"""

import json
from typing import List, Optional, Dict, Any
import urllib.parse

# Import the generated WIT bindings
from wit_world.exports import ToolHandler
from wit_world.imports import tools, fastertools_mcp_types as types
from wit_world.imports import wasi_http_types as http_types, outgoing_handler, streams, poll
from wit_world.types import Some, Err


class ToolHandler(ToolHandler):
    """
    MCP Tool Handler implementation using WIT bindings.
    """
    
    def __init__(self):
        # Define our tools with their schemas
        self.tool_definitions = {
            "echo": {
                "description": "Echo a message back to the user",
                "schema": {
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "The message to echo"
                        }
                    },
                    "required": ["message"]
                }
            },
            "get_weather": {
                "description": "Get current weather for a location",
                "schema": {
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "City name to get weather for"
                        }
                    },
                    "required": ["location"]
                }
            },
            "multi_weather": {
                "description": "Get weather for multiple cities concurrently",
                "schema": {
                    "type": "object",
                    "properties": {
                        "cities": {
                            "type": "array",
                            "description": "List of cities to get weather for",
                            "items": {
                                "type": "string"
                            },
                            "minItems": 1,
                            "maxItems": 5
                        }
                    },
                    "required": ["cities"]
                }
            }
        }
    
    def handle_list_tools(self, request: tools.ListToolsRequest) -> tools.ListToolsResponse:
        """List available tools."""
        tool_list = []
        
        for name, definition in self.tool_definitions.items():
            tool = tools.Tool(
                base=types.BaseMetadata(
                    name=name,
                    title=name
                ),
                description=definition["description"],
                input_schema=json.dumps(definition["schema"]),
                output_schema=None,
                annotations=None,
                meta=None
            )
            tool_list.append(tool)
        
        return tools.ListToolsResponse(
            tools=tool_list,
            next_cursor=None,
            meta=None
        )
    
    def handle_call_tool(self, request: tools.CallToolRequest) -> tools.ToolResult:
        """Execute a tool."""
        try:
            # Parse arguments if they're a string
            args = {}
            if request.arguments:
                args = json.loads(request.arguments)
            
            # Route to the appropriate tool handler
            if request.name == "echo":
                result = self._handle_echo(args)
            elif request.name == "get_weather":
                result = self._handle_weather(args)
            elif request.name == "multi_weather":
                result = self._handle_multi_weather(args)
            else:
                result = f"Unknown tool: {request.name}"
                return self._create_error_result(result)
            
            # Return success result
            return self._create_text_result(result)
            
        except Exception as e:
            return self._create_error_result(f"Error executing {request.name}: {str(e)}")
    
    def _handle_echo(self, args: dict) -> str:
        """Handle echo tool."""
        message = args.get("message")
        if not message:
            raise ValueError("Missing required field: message")
        return f"Echo: {message}"
    
    def _handle_weather(self, args: dict) -> str:
        """Handle get_weather tool."""
        location = args.get("location")
        if not location:
            raise ValueError("Missing required field: location")
        
        try:
            # Use synchronous version for simplicity in componentize-py
            return self._get_weather_for_city_sync(location)
        except Exception as e:
            return f"Error fetching weather: {str(e)}"
    
    def _handle_multi_weather(self, args: dict) -> str:
        """Handle multi_weather tool."""
        cities = args.get("cities")
        
        if not cities or not isinstance(cities, list):
            raise ValueError("Missing or invalid cities field")
        
        if len(cities) == 0:
            raise ValueError("No cities provided")
        
        if len(cities) > 5:
            raise ValueError("Maximum 5 cities allowed")
        
        # Execute weather fetches sequentially (componentize-py doesn't support async well yet)
        output = "=== Concurrent Weather Results ===\n\n"
        
        for city in cities:
            try:
                weather = self._get_weather_for_city_sync(city)
                output += f"{weather}\n\n"
            except Exception as e:
                output += f"Error fetching weather for {city}: {str(e)}\n\n"
        
        output += "=== All requests completed ==="
        
        return output
    
    def _get_weather_for_city_sync(self, location: str) -> str:
        """Synchronous version of weather fetching using WASI HTTP."""
        try:
            # Geocode the location
            geocoding_url = f"https://geocoding-api.open-meteo.com/v1/search?name={urllib.parse.quote(location)}&count=1"
            
            geo_data = self._http_get(geocoding_url)
            
            if not geo_data.get("results"):
                raise Exception(f"Location '{location}' not found")
            
            location_data = geo_data["results"][0]
            
            # Fetch the weather
            weather_url = (
                f"https://api.open-meteo.com/v1/forecast?"
                f"latitude={location_data['latitude']}&longitude={location_data['longitude']}"
                f"&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code"
            )
            
            weather_data = self._http_get(weather_url)
            
            conditions = self._get_weather_condition(weather_data["current"]["weather_code"])
            
            return (
                f"Weather in {location_data['name']}, {location_data['country']}:\n"
                f"Temperature: {weather_data['current']['temperature_2m']:.1f}°C "
                f"(feels like {weather_data['current']['apparent_temperature']:.1f}°C)\n"
                f"Conditions: {conditions}\n"
                f"Humidity: {weather_data['current']['relative_humidity_2m']}%\n"
                f"Wind: {weather_data['current']['wind_speed_10m']:.1f} km/h"
            )
            
        except Exception as e:
            raise Exception(f"Failed to fetch weather for {location}: {str(e)}")
    
    def _http_get(self, url: str) -> Dict[str, Any]:
        """Make an HTTP GET request using WASI HTTP."""
        # Parse the URL
        if not url.startswith(('http://', 'https://')):
            raise ValueError(f"Invalid URL scheme: {url}")
        
        # Extract components from URL
        if url.startswith('https://'):
            scheme = http_types.Scheme_Https()
            host_start = 8
        else:
            scheme = http_types.Scheme_Http()
            host_start = 7
        
        # Find host and path
        path_start = url.find('/', host_start)
        if path_start == -1:
            host = url[host_start:]
            path_and_query = "/"
        else:
            host = url[host_start:path_start]
            path_and_query = url[path_start:]
        
        # Create the request headers (minimal headers only)
        headers = http_types.Fields.from_list([
            ("Accept", b"application/json")
        ])
        
        request = http_types.OutgoingRequest(headers)
        request.set_scheme(Some(value=scheme))
        request.set_authority(Some(value=host))
        request.set_path_with_query(Some(value=path_and_query))
        
        # Send the request
        future_response = outgoing_handler.handle(request, None)
        
        # Poll for the response
        pollable = future_response.subscribe()
        
        # Wait for the response to be ready
        while True:
            poll_list = [pollable]
            ready = poll.poll(poll_list)
            if ready:
                break
        
        # Get the response
        response_result = future_response.get()
        
        if response_result is None:
            # Still pending, poll again
            raise Exception("Response still pending after poll")
        
        # Check if we got an error
        if isinstance(response_result, Err):
            error_context = response_result.value
            if hasattr(error_context, 'message'):
                raise Exception(f"HTTP request failed: {error_context.message}")
            else:
                raise Exception(f"HTTP request failed with error")
        
        # Extract the Ok value
        if hasattr(response_result, 'value'):
            incoming_response = response_result.value
        else:
            incoming_response = response_result
        
        # Get the response body
        body_result = incoming_response.consume()
        
        if isinstance(body_result, Err):
            raise Exception(f"Failed to get response body")
        
        # Extract the body stream
        if hasattr(body_result, 'value'):
            body_stream = body_result.value
        else:
            body_stream = body_result
        
        # Read the body
        body_parts = []
        while True:
            # Use blocking_read method on the stream
            chunk = body_stream.blocking_read(8192)
            
            if not chunk:
                break
            body_parts.append(chunk)
        
        # Combine the body parts
        body = b''.join(body_parts)
        
        # Parse JSON response
        return json.loads(body.decode('utf-8'))
    
    def _get_weather_condition(self, code: int) -> str:
        """Get weather condition description from WMO code."""
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
            56: "Light freezing drizzle",
            57: "Dense freezing drizzle",
            61: "Slight rain",
            63: "Moderate rain",
            65: "Heavy rain",
            66: "Light freezing rain",
            67: "Heavy freezing rain",
            71: "Slight snow fall",
            73: "Moderate snow fall",
            75: "Heavy snow fall",
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
    
    def _create_text_result(self, text: str) -> tools.ToolResult:
        """Create a text result in MCP format."""
        content_block = types.ContentBlock_Text(
            value=types.TextContent(
                text=text,
                annotations=None,
                meta=None
            )
        )
        
        return tools.ToolResult(
            content=[content_block],
            structured_content=None,
            is_error=False,
            meta=None
        )
    
    def _create_error_result(self, message: str) -> tools.ToolResult:
        """Create an error result in MCP format."""
        content_block = types.ContentBlock_Text(
            value=types.TextContent(
                text=message,
                annotations=None,
                meta=None
            )
        )
        
        return tools.ToolResult(
            content=[content_block],
            structured_content=None,
            is_error=True,
            meta=None
        )


# For testing without component compilation
if __name__ == "__main__":
    # Test the component locally
    component = ToolHandler()
    
    # Test list tools
    print("Testing list tools...")
    list_request = tools.ListToolsRequest(cursor=None, progress_token=None, meta=None)
    list_response = component.handle_list_tools(list_request)
    for tool in list_response.tools:
        print(f"  - {tool.base.name}: {tool.description}")
    
    # Test echo tool
    print("\nTesting echo tool...")
    echo_request = tools.CallToolRequest(
        name="echo",
        arguments='{"message": "Hello World"}',
        progress_token=None,
        meta=None
    )
    echo_result = component.handle_call_tool(echo_request)
    print(f"  Result: {echo_result.content[0].value.text}")
    
    # Test weather tool
    print("\nTesting weather tool...")
    weather_request = tools.CallToolRequest(
        name="get_weather",
        arguments='{"location": "London"}',
        progress_token=None,
        meta=None
    )
    weather_result = component.handle_call_tool(weather_request)
    print(f"  Result:\n{weather_result.content[0].value.text}")