"""
Python MCP Weather Handler

Clean implementation using wasi-http-async SDK for HTTP requests.
Implements the same tools as the Rust and JavaScript examples:
- echo: Echo a message back
- get_weather: Get weather for a single location  
- multi_weather: Get weather for multiple locations concurrently
"""

import json
import asyncio
import urllib.parse
from typing import List, Dict, Any

# Import MCP types
from wit_world.exports import ToolHandler
from wit_world.imports import tools, fastertools_mcp_types as types
from wit_world.types import Ok, Err

# Import WASI types for the SDK
from wit_world.imports import wasi_http_types as http_types, outgoing_handler, poll, streams
from wit_world.imports.streams import StreamError_Closed
from wit_world.imports.wasi_http_types import IncomingBody

# Patch the SDK's bindings (temporary until SDK has better discovery)
import wasi_http_async.bindings as bindings_module
bindings_module.bindings._http_types = http_types
bindings_module.bindings._outgoing_handler = outgoing_handler
bindings_module.bindings._poll = poll
bindings_module.bindings._streams = streams
bindings_module.bindings._loaded = True

import wasi_http_async.stream as stream_module
stream_module.Err = Err
stream_module.StreamError_Closed = StreamError_Closed
stream_module.IncomingBody = IncomingBody

import wasi_http_async.core as core_module
core_module.Ok = Ok
core_module.Err = Err

# Now import from the SDK
from wasi_http_async import fetch
from wasi_http_async.poll_loop import PollLoop


class ToolHandler(ToolHandler):
    """MCP Tool Handler implementation."""
    
    def handle_list_tools(self, request: tools.ListToolsRequest) -> tools.ListToolsResponse:
        """List available tools."""
        return tools.ListToolsResponse(
            tools=[
                tools.Tool(
                    base=types.BaseMetadata(name="echo", title="echo"),
                    description="Echo a message back to the user",
                    input_schema=json.dumps({
                        "type": "object",
                        "properties": {
                            "message": {"type": "string", "description": "The message to echo"}
                        },
                        "required": ["message"]
                    }),
                    output_schema=None,
                    annotations=None,
                    meta=None
                ),
                tools.Tool(
                    base=types.BaseMetadata(name="get_weather", title="get_weather"),
                    description="Get current weather for a location",
                    input_schema=json.dumps({
                        "type": "object",
                        "properties": {
                            "location": {"type": "string", "description": "City name to get weather for"}
                        },
                        "required": ["location"]
                    }),
                    output_schema=None,
                    annotations=None,
                    meta=None
                ),
                tools.Tool(
                    base=types.BaseMetadata(name="multi_weather", title="multi_weather"),
                    description="Get weather for multiple cities concurrently",
                    input_schema=json.dumps({
                        "type": "object",
                        "properties": {
                            "cities": {
                                "type": "array",
                                "description": "List of cities to get weather for",
                                "items": {"type": "string"}
                            }
                        },
                        "required": ["cities"]
                    }),
                    output_schema=None,
                    annotations=None,
                    meta=None
                )
            ],
            next_cursor=None,
            meta=None
        )
    
    def handle_call_tool(self, request: tools.CallToolRequest) -> tools.ToolResult:
        """Execute a tool."""
        try:
            args = json.loads(request.arguments) if request.arguments else {}
            
            if request.name == "echo":
                return self._handle_echo(args)
            elif request.name == "get_weather":
                return self._handle_weather(args)
            elif request.name == "multi_weather":
                return self._handle_multi_weather(args)
            else:
                return self._error(f"Unknown tool: {request.name}")
                
        except Exception as e:
            return self._error(f"Tool execution failed: {str(e)}")
    
    def _handle_echo(self, args: dict) -> tools.ToolResult:
        """Handle echo tool."""
        message = args.get("message", "")
        return self._success(f"Echo: {message}")
    
    def _handle_weather(self, args: dict) -> tools.ToolResult:
        """Handle get_weather tool."""
        location = args.get("location")
        if not location:
            return self._error("Missing 'location' argument")
        
        loop = PollLoop()
        try:
            weather_data = loop.run_until_complete(self._fetch_weather(location))
            return self._success(self._format_weather(weather_data))
        except Exception as e:
            return self._error(f"Weather fetch failed: {str(e)}")
        finally:
            loop.close()
    
    def _handle_multi_weather(self, args: dict) -> tools.ToolResult:
        """Handle multi_weather tool."""
        cities = args.get("cities", [])
        if not cities:
            return self._error("Missing 'cities' argument")
        
        loop = PollLoop()
        try:
            results = loop.run_until_complete(self._fetch_multi_weather(cities))
            formatted = [self._format_weather(r) if not isinstance(r, Exception) else {"error": str(r)} for r in results]
            return self._success(json.dumps(formatted, indent=2))
        except Exception as e:
            return self._error(f"Multi-weather fetch failed: {str(e)}")
        finally:
            loop.close()
    
    async def _fetch_weather(self, city: str) -> Dict[str, Any]:
        """Fetch weather data for a single city."""
        # Geocode
        geo_url = f"https://geocoding-api.open-meteo.com/v1/search?name={urllib.parse.quote(city)}&count=1"
        response = await fetch(geo_url)
        geo_data = await response.json()
        
        if not geo_data.get("results"):
            raise Exception(f"Location '{city}' not found")
        
        location = geo_data["results"][0]
        
        # Get weather
        weather_url = (
            f"https://api.open-meteo.com/v1/forecast?"
            f"latitude={location['latitude']}&longitude={location['longitude']}"
            f"&current=temperature_2m,relative_humidity_2m,wind_speed_10m,weather_code"
        )
        response = await fetch(weather_url)
        weather = await response.json()
        
        return {
            "name": location["name"],
            "country": location["country"],
            "temperature": weather["current"]["temperature_2m"],
            "humidity": weather["current"]["relative_humidity_2m"],
            "wind_speed": weather["current"]["wind_speed_10m"],
            "weather_code": weather["current"]["weather_code"]
        }
    
    async def _fetch_multi_weather(self, cities: List[str]) -> List[Any]:
        """Fetch weather for multiple cities concurrently."""
        tasks = [self._fetch_weather(city) for city in cities]
        return await asyncio.gather(*tasks, return_exceptions=True)
    
    def _format_weather(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """Format weather data for display."""
        return {
            "location": f"{data['name']}, {data['country']}",
            "temperature": f"{data['temperature']:.1f}°C",
            "conditions": f"Weather code {data['weather_code']}",
            "humidity": f"{data['humidity']}%",
            "wind": f"{data['wind_speed']:.1f} m/s"
        }
    
    def _success(self, text: str) -> tools.ToolResult:
        """Create a success result."""
        return tools.ToolResult(
            content=[types.ContentBlock_Text(
                value=types.TextContent(text=text, annotations=None, meta=None)
            )],
            structured_content=None,
            is_error=False,
            meta=None
        )
    
    def _error(self, text: str) -> tools.ToolResult:
        """Create an error result."""
        return tools.ToolResult(
            content=[types.ContentBlock_Text(
                value=types.TextContent(text=text, annotations=None, meta=None)
            )],
            structured_content=None,
            is_error=True,
            meta=None
        )