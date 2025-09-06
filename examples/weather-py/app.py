"""
Python MCP Weather Server - Transparent Implementation

A WebAssembly component that provides weather tools using the WIT bindings directly.
This implementation prioritizes transparency and understanding over abstraction.

Implements three example tools:
- echo: Echo a message back
- get_weather: Get weather for a single location
- multi_weather: Get weather for multiple locations concurrently
"""

import json
import asyncio
import urllib.parse
from typing import List, Dict, Any, Optional

# Import componentize-py's built-in async HTTP support
import poll_loop
from poll_loop import PollLoop, Stream

# Import the generated WIT bindings directly - these are our SDK
from wit_world.exports import ToolsCapabilities, CoreCapabilities
from wit_world.imports import core_types, tool_types, fastertools_mcp_types as mcp_types
from wit_world.imports.types import (
    OutgoingRequest,
    Fields,
    Scheme_Http,
    Scheme_Https,
    Method_Get,
)


class WeatherMCPCapabilities(ToolsCapabilities, CoreCapabilities):
    """
    Direct implementation of the MCP capabilities interfaces.
    This class is what componentize-py expects to find.
    """

    # Tool definitions as class data
    TOOLS = {
        "echo": {
            "description": "Echo a message back to the user",
            "schema": {
                "type": "object",
                "properties": {
                    "message": {"type": "string", "description": "Message to echo"}
                },
                "required": ["message"],
            },
        },
        "get_weather": {
            "description": "Get current weather for a location",
            "schema": {
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "City name or location",
                    }
                },
                "required": ["location"],
            },
        },
        "multi_weather": {
            "description": "Get weather for multiple cities concurrently",
            "schema": {
                "type": "object",
                "properties": {
                    "cities": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of city names (max 5)",
                    }
                },
                "required": ["cities"],
            },
        },
    }

    # -------------------------------------------------------------------------
    # Core Capabilities (session management)
    # -------------------------------------------------------------------------

    def handle_initialize(
        self, request: core_types.InitializeRequest
    ) -> core_types.InitializeResponse:
        """Handle the MCP initialize request."""
        return core_types.InitializeResponse(
            protocol_version=core_types.ProtocolVersion.V20250618,
            capabilities=core_types.ServerCapabilities(
                tools=core_types.ToolsCapability(list_changed=None),
                experimental=None,
                logging=None,
                completions=None,
                prompts=None,
                resources=None,
            ),
            server_info=core_types.ImplementationInfo(
                name="weather-py", version="0.1.0", title="weather-py"
            ),
            instructions="An MCP server written in Python",
            meta=None,
        )

    def handle_initialized(self) -> None:
        """Called after successful initialization."""
        return None

    def handle_ping(self) -> None:
        """Handle keepalive ping."""
        return None

    def handle_shutdown(self) -> None:
        """Handle graceful shutdown."""
        return None

    def get_auth_config(self) -> Optional:
        """Return auth configuration if this server requires authentication."""
        return None  # No auth required for this example
        # Uncomment and configure for OAuth authentication:
        # from wit_world.imports.authorization_types import ProviderAuthConfig
        # return ProviderAuthConfig(
        #     expected_issuer="https://xxxxx.authkit.app",
        #     expected_audiences=["client_xxxxx"],
        #     jwks_uri="https://xxxxx.authkit.app/oauth2/jwks",
        #     policy=None,  # Optional: Add Rego policy for additional authorization
        #     policy_data=None,  # Optional: Add policy data as JSON string
        # )

    def jwks_cache_get(self, jwks_uri: str) -> Optional[str]:
        """Optional: Get cached JWKS for the given URI."""
        return None  # Not implementing caching

    def jwks_cache_set(self, jwks_uri: str, jwks: str) -> None:
        """Optional: Cache JWKS for the given URI."""
        pass  # Not implementing caching

    # -------------------------------------------------------------------------
    # Tools Capabilities
    # -------------------------------------------------------------------------

    def handle_list_tools(
        self, request: tool_types.ListToolsRequest
    ) -> tool_types.ListToolsResponse:
        """Return the list of available tools."""
        tools = []

        for name, metadata in self.TOOLS.items():
            tools.append(
                tool_types.Tool(
                    base=mcp_types.BaseMetadata(name=name, title=name),
                    description=metadata["description"],
                    input_schema=json.dumps(metadata["schema"]),
                    output_schema=None,
                    annotations=None,
                    meta=None,
                )
            )

        return tool_types.ListToolsResponse(tools=tools, next_cursor=None, meta=None)

    def handle_call_tool(
        self, request: tool_types.CallToolRequest
    ) -> tool_types.ToolResult:
        """Execute a tool and return the result."""
        # Parse arguments
        arguments = {}
        if request.arguments:
            try:
                arguments = json.loads(request.arguments)
            except json.JSONDecodeError as e:
                return self._error_result(f"Invalid JSON arguments: {e}")

        # Route to the appropriate tool
        try:
            if request.name == "echo":
                result_text = self._execute_echo(arguments)
            elif request.name == "get_weather":
                result_text = self._execute_weather_sync(arguments)
            elif request.name == "multi_weather":
                result_text = self._execute_multi_weather_sync(arguments)
            else:
                return self._error_result(f"Unknown tool: {request.name}")

            return self._text_result(result_text)

        except Exception as e:
            return self._error_result(f"Tool execution failed: {str(e)}")

    # -------------------------------------------------------------------------
    # Tool implementations
    # -------------------------------------------------------------------------

    def _execute_echo(self, arguments: dict) -> str:
        """Simple synchronous echo tool."""
        message = arguments.get("message", "")
        return f"Echo: {message}"

    def _execute_weather_sync(self, arguments: dict) -> str:
        """Get weather for a single location."""
        location = arguments.get("location", "")

        # Run async code in PollLoop
        loop = PollLoop()
        asyncio.set_event_loop(loop)
        try:
            weather_data = loop.run_until_complete(self._fetch_weather(location))
            return self._format_weather(weather_data)
        finally:
            loop.close()

    def _execute_multi_weather_sync(self, arguments: dict) -> str:
        """Get weather for multiple cities concurrently."""
        cities = arguments.get("cities", [])

        if not cities:
            return "No cities provided"
        if len(cities) > 5:
            return "Maximum 5 cities allowed"

        # Run async code in PollLoop
        loop = PollLoop()
        asyncio.set_event_loop(loop)
        try:
            return loop.run_until_complete(self._fetch_multi_weather(cities))
        finally:
            loop.close()

    async def _fetch_multi_weather(self, cities: List[str]) -> str:
        """Fetch weather for multiple cities concurrently."""
        tasks = [self._fetch_weather(city) for city in cities]
        results = await asyncio.gather(*tasks, return_exceptions=True)

        output = "=== Weather Results ===\n\n"
        for city, result in zip(cities, results):
            if isinstance(result, Exception):
                output += f"Error fetching weather for {city}: {result}\n\n"
            else:
                output += self._format_weather(result) + "\n\n"
        output += "=== All requests completed ==="

        return output

    async def _fetch_weather(self, city: str) -> Dict[str, Any]:
        """Fetch weather data for a single city."""
        # Geocode the location
        geo_url = f"https://geocoding-api.open-meteo.com/v1/search?name={urllib.parse.quote(city)}&count=1"
        geo_data = await self._fetch_json(geo_url)

        if not geo_data.get("results"):
            raise Exception(f"Location '{city}' not found")

        location = geo_data["results"][0]

        # Get weather data
        weather_url = (
            f"https://api.open-meteo.com/v1/forecast?"
            f"latitude={location['latitude']}&longitude={location['longitude']}"
            f"&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code"
        )
        weather = await self._fetch_json(weather_url)

        return {
            "name": location["name"],
            "country": location["country"],
            "temperature": weather["current"]["temperature_2m"],
            "apparent_temperature": weather["current"]["apparent_temperature"],
            "humidity": weather["current"]["relative_humidity_2m"],
            "wind_speed": weather["current"]["wind_speed_10m"],
            "weather_code": weather["current"]["weather_code"],
        }

    async def _fetch_json(self, url: str) -> dict:
        """Fetch JSON from a URL using WASI HTTP."""
        parsed = urllib.parse.urlparse(url)

        # Create HTTP request
        request = OutgoingRequest(Fields.from_list([]))

        if parsed.scheme == "https":
            request.set_scheme(Scheme_Https())
        else:
            request.set_scheme(Scheme_Http())

        request.set_authority(parsed.netloc)

        path_with_query = parsed.path
        if parsed.query:
            path_with_query += f"?{parsed.query}"
        request.set_path_with_query(path_with_query)

        request.set_method(Method_Get())

        # Send request
        response = await poll_loop.send(request)

        # Check status
        status = response.status()
        if status < 200 or status >= 300:
            raise Exception(f"HTTP {status} from {url}")

        # Read response body
        stream = Stream(response.consume())
        chunks = []
        while True:
            chunk = await stream.next()
            if chunk is None:
                break
            chunks.append(chunk)

        body = b"".join(chunks)
        return json.loads(body)

    def _format_weather(self, data: Dict[str, Any]) -> str:
        """Format weather data as human-readable text."""
        return (
            f"Weather in {data['name']}, {data['country']}:\n"
            f"Temperature: {data['temperature']:.1f}°C "
            f"(feels like {data['apparent_temperature']:.1f}°C)\n"
            f"Conditions: {self._weather_condition(data['weather_code'])}\n"
            f"Humidity: {data['humidity']}%\n"
            f"Wind: {data['wind_speed']:.1f} km/h"
        )

    def _weather_condition(self, code: int) -> str:
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
            85: "Slight snow showers",
            86: "Heavy snow showers",
            95: "Thunderstorm",
            96: "Thunderstorm with slight hail",
            99: "Thunderstorm with heavy hail",
        }
        return conditions.get(code, "Unknown")

    # -------------------------------------------------------------------------
    # Helper methods for creating MCP results
    # -------------------------------------------------------------------------

    def _text_result(self, text: str) -> tool_types.ToolResult:
        """Create a successful text result."""
        return tool_types.ToolResult(
            content=[
                mcp_types.ContentBlock_Text(
                    value=mcp_types.TextContent(text=text, annotations=None, meta=None)
                )
            ],
            structured_content=None,
            is_error=False,
            meta=None,
        )

    def _error_result(self, message: str) -> tool_types.ToolResult:
        """Create an error result."""
        return tool_types.ToolResult(
            content=[
                mcp_types.ContentBlock_Text(
                    value=mcp_types.TextContent(
                        text=message, annotations=None, meta=None
                    )
                )
            ],
            structured_content=None,
            is_error=True,
            meta=None,
        )


# Export for componentize-py - it expects these class names
ToolsCapabilities = WeatherMCPCapabilities
CoreCapabilities = WeatherMCPCapabilities
