"""
Python MCP Weather Provider - v2 Architecture
Implements core-capabilities for full control over identity and initialization
"""

import json
from typing import List, Dict, Any, Optional
from dataclasses import dataclass

# WIT bindings would be generated, this is pseudocode
from mcp_bindings import (
    CoreCapabilities,
    ToolsCapabilities,
    InitializeRequest,
    InitializeResponse,
    ImplementationInfo,
    ServerCapabilities,
    ToolsCapability,
    ProtocolVersion,
    Tool,
    ToolResult,
    TextContent,
    ContentBlock,
    ListToolsRequest,
    ListToolsResponse,
    CallToolRequest,
    McpError
)


class WeatherProvider(CoreCapabilities, ToolsCapabilities):
    """
    A complete MCP provider that owns its identity and capabilities.
    No external configuration needed - everything is in code.
    """
    
    def __init__(self):
        self.name = "weather-py"
        self.version = "2.0.0"
        self.title = "Python Weather Information Service"
        self.instructions = """
        Get real-time weather for any city worldwide.
        
        Available tools:
        - get_weather: Get current weather for a single location
        - get_forecast: Get 5-day forecast
        - compare_weather: Compare weather between multiple cities
        
        Example: get_weather({"city": "London"})
        """
        
    # === CORE CAPABILITIES (MANDATORY) ===
    
    def handle_initialize(self, request: InitializeRequest) -> InitializeResponse:
        """
        Provider controls its complete identity and capability declaration.
        No hardcoded transport values, no config files needed.
        """
        return InitializeResponse(
            protocol_version=ProtocolVersion.MCP_V20250618,
            server_info=ImplementationInfo(
                name=self.name,
                version=self.version,
                title=self.title
            ),
            capabilities=ServerCapabilities(
                tools=ToolsCapability(
                    list_changed=False  # We don't dynamically change tools
                ),
                # Could add more capabilities here if we supported them
                resources=None,
                prompts=None,
                sampling=None
            ),
            instructions=self.instructions
        )
    
    def handle_initialized(self) -> None:
        """Notification that initialization is complete."""
        # Could set up resources, start background tasks, etc.
        print(f"{self.name} v{self.version} initialized successfully")
        return None
    
    def handle_ping(self) -> None:
        """Keep-alive ping."""
        return None
    
    def handle_shutdown(self) -> None:
        """Clean shutdown."""
        # Could close connections, save state, etc.
        print(f"{self.name} shutting down gracefully")
        return None
    
    # === TOOLS CAPABILITIES ===
    
    def handle_list_tools(self, request: ListToolsRequest) -> ListToolsResponse:
        """List our available weather tools."""
        tools = [
            Tool(
                name="get_weather",
                description="Get current weather for a city",
                input_schema=json.dumps({
                    "type": "object",
                    "properties": {
                        "city": {
                            "type": "string",
                            "description": "City name"
                        },
                        "units": {
                            "type": "string",
                            "enum": ["celsius", "fahrenheit"],
                            "default": "celsius"
                        }
                    },
                    "required": ["city"]
                })
            ),
            Tool(
                name="get_forecast",
                description="Get 5-day weather forecast",
                input_schema=json.dumps({
                    "type": "object",
                    "properties": {
                        "city": {
                            "type": "string",
                            "description": "City name"
                        },
                        "days": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 5,
                            "default": 5
                        }
                    },
                    "required": ["city"]
                })
            ),
            Tool(
                name="compare_weather",
                description="Compare weather between multiple cities",
                input_schema=json.dumps({
                    "type": "object",
                    "properties": {
                        "cities": {
                            "type": "array",
                            "items": {"type": "string"},
                            "minItems": 2,
                            "maxItems": 5
                        }
                    },
                    "required": ["cities"]
                })
            )
        ]
        
        return ListToolsResponse(
            tools=tools,
            next_cursor=None  # No pagination needed
        )
    
    def handle_call_tool(self, request: CallToolRequest) -> ToolResult:
        """Execute a weather tool."""
        try:
            args = json.loads(request.arguments) if request.arguments else {}
            
            if request.name == "get_weather":
                result = self._get_weather(
                    args["city"],
                    args.get("units", "celsius")
                )
            elif request.name == "get_forecast":
                result = self._get_forecast(
                    args["city"],
                    args.get("days", 5)
                )
            elif request.name == "compare_weather":
                result = self._compare_weather(args["cities"])
            else:
                raise ValueError(f"Unknown tool: {request.name}")
            
            return ToolResult(
                content=[ContentBlock.text(TextContent(text=result))],
                is_error=False
            )
            
        except Exception as e:
            return ToolResult(
                content=[ContentBlock.text(TextContent(
                    text=f"Error: {str(e)}"
                ))],
                is_error=True
            )
    
    # === INTERNAL IMPLEMENTATION ===
    
    def _get_weather(self, city: str, units: str) -> str:
        """Actual weather fetching logic."""
        # This would call a real weather API
        temp = 22 if units == "celsius" else 72
        unit_symbol = "°C" if units == "celsius" else "°F"
        
        return f"""
        Weather for {city}:
        Temperature: {temp}{unit_symbol}
        Conditions: Partly cloudy
        Humidity: 65%
        Wind: 10 km/h NW
        """
    
    def _get_forecast(self, city: str, days: int) -> str:
        """Get forecast data."""
        forecasts = []
        for i in range(days):
            forecasts.append(f"Day {i+1}: Sunny, 22°C / 15°C")
        
        return f"5-day forecast for {city}:\n" + "\n".join(forecasts)
    
    def _compare_weather(self, cities: List[str]) -> str:
        """Compare weather between cities."""
        comparisons = []
        for city in cities:
            comparisons.append(f"{city}: 22°C, Partly cloudy")
        
        return "Weather comparison:\n" + "\n".join(comparisons)


# For componentize-py or other bindings
Provider = WeatherProvider