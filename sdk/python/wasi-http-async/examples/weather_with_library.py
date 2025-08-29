#!/usr/bin/env python3
"""
Example MCP weather component using the wasi-http-async library.

This shows how much simpler the code becomes with the library.
"""

import asyncio
import json
import sys
import os

# In production, this would be: from wasi_http_async import fetch
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'src'))
from wasi_http_async import fetch


async def get_weather(city: str) -> dict:
    """Get weather for a city using OpenWeatherMap API."""
    api_key = os.environ.get("OPENWEATHER_API_KEY", "demo_key")
    url = f"https://api.openweathermap.org/data/2.5/weather?q={city}&appid={api_key}&units=metric"
    
    response = await fetch(url)
    if not response.ok:
        raise Exception(f"Weather API error: {response.status}")
    
    data = await response.json()
    return {
        "city": data["name"],
        "temperature": data["main"]["temp"],
        "description": data["weather"][0]["description"],
        "humidity": data["main"]["humidity"],
        "wind_speed": data["wind"]["speed"]
    }


async def get_multi_weather(cities: list) -> list:
    """Get weather for multiple cities concurrently."""
    tasks = [get_weather(city) for city in cities]
    return await asyncio.gather(*tasks, return_exceptions=True)


# MCP Handler
class Handler:
    def handle(self, request):
        """Main MCP request handler."""
        
        if request["method"] == "tools/list":
            return {
                "tools": [
                    {
                        "name": "get_weather",
                        "description": "Get current weather for a city",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "city": {"type": "string", "description": "City name"}
                            },
                            "required": ["city"]
                        }
                    },
                    {
                        "name": "multi_weather",
                        "description": "Get weather for multiple cities",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "cities": {
                                    "type": "array",
                                    "items": {"type": "string"},
                                    "description": "List of city names"
                                }
                            },
                            "required": ["cities"]
                        }
                    }
                ]
            }
        
        elif request["method"] == "tools/call":
            tool_name = request["params"]["name"]
            arguments = request["params"].get("arguments", {})
            
            # Run async function in event loop
            from wasi_http_async.poll_loop import PollLoop
            loop = PollLoop()
            
            try:
                if tool_name == "get_weather":
                    result = loop.run_until_complete(
                        get_weather(arguments["city"])
                    )
                elif tool_name == "multi_weather":
                    result = loop.run_until_complete(
                        get_multi_weather(arguments["cities"])
                    )
                else:
                    return {"error": f"Unknown tool: {tool_name}"}
                
                return {"content": [{"type": "text", "text": json.dumps(result, indent=2)}]}
                
            except Exception as e:
                return {"error": str(e)}
            finally:
                loop.close()
        
        return {"error": "Unknown method"}


# Componentize-py expects a handle function
def handle(request):
    handler = Handler()
    return handler.handle(request)


# Compare line counts:
# Original app.py: ~250 lines of boilerplate + logic
# This version: ~100 lines of just logic
# Reduction: 60% less code!