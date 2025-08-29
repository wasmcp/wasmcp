"""
Python MCP Weather Handler

Demonstrates the clean helper API for building MCP tools in Python.
Implements the same tools as the Rust and JavaScript examples:
- echo: Echo a message back
- get_weather: Get weather for a single location
- multi_weather: Get weather for multiple locations concurrently
"""

import json
import asyncio
from typing import List, Optional, Dict, Any
import urllib.parse
from urllib.parse import urlparse

# Import the generated WIT bindings
from wit_world.exports import ToolHandler
from wit_world.imports import tools, fastertools_mcp_types as types
from wit_world.imports import wasi_http_types as http_types, outgoing_handler, streams, poll
from wit_world.imports.streams import StreamError_Closed
from wit_world.imports.wasi_http_types import IncomingBody
from wit_world.types import Err, Ok

# Copy Spin SDK's PollLoop implementation
class PollLoop(asyncio.AbstractEventLoop):
    """Custom asyncio event loop backed by wasi:io/poll#poll."""
    
    def __init__(self):
        self.wakers = []
        self.running = False
        self.handles = []
        self.exception = None

    def get_debug(self):
        return False

    def run_until_complete(self, future):
        future = asyncio.ensure_future(future, loop=self)
        self.running = True
        asyncio.events._set_running_loop(self)
        while self.running and not future.done():
            handles = self.handles
            self.handles = []
            for handle in handles:
                if not handle._cancelled:
                    handle._run()
            
            if self.wakers:
                pollables, wakers = list(map(list, zip(*self.wakers)))
                new_wakers = []
                ready = [False] * len(pollables)
                for index in poll.poll(pollables):
                    ready[index] = True
                
                for (ready, pollable), waker in zip(zip(ready, pollables), wakers):
                    if ready:
                        pollable.__exit__(None, None, None)
                        waker.set_result(None)
                    else:
                        new_wakers.append((pollable, waker))
                
                self.wakers = new_wakers
            
            if self.exception is not None:
                raise self.exception
        
        return future.result()

    def is_running(self):
        return self.running

    def is_closed(self):
        return not self.running

    def stop(self):
        self.running = False

    def close(self):
        self.running = False

    def call_soon(self, callback, *args, context=None):
        handle = asyncio.Handle(callback, args, self, context)
        self.handles.append(handle)
        return handle

    def create_task(self, coroutine):
        return asyncio.Task(coroutine, loop=self)

    def create_future(self):
        return asyncio.Future(loop=self)

    def shutdown_asyncgens(self):
        pass

    def call_exception_handler(self, context):
        self.exception = context.get('exception', None)

    # Stub out other required methods
    def time(self): raise NotImplementedError
    def call_later(self, delay, callback, *args, context=None): raise NotImplementedError
    def call_at(self, when, callback, *args, context=None): raise NotImplementedError
    def run_forever(self): raise NotImplementedError
    def call_soon_threadsafe(self, callback, *args, context=None): raise NotImplementedError
    def run_in_executor(self, executor, func, *args): raise NotImplementedError
    def set_default_executor(self, executor): raise NotImplementedError
    async def getaddrinfo(self, host, port, *, family=0, type=0, proto=0, flags=0): raise NotImplementedError
    async def getnameinfo(self, sockaddr, flags=0): raise NotImplementedError
    async def create_connection(self, *args, **kwargs): raise NotImplementedError
    async def create_server(self, *args, **kwargs): raise NotImplementedError
    async def sendfile(self, *args, **kwargs): raise NotImplementedError
    async def start_tls(self, *args, **kwargs): raise NotImplementedError
    async def create_unix_connection(self, *args, **kwargs): raise NotImplementedError
    async def create_unix_server(self, *args, **kwargs): raise NotImplementedError
    async def connect_accepted_socket(self, *args, **kwargs): raise NotImplementedError
    async def create_datagram_endpoint(self, *args, **kwargs): raise NotImplementedError
    async def connect_read_pipe(self, protocol_factory, pipe): raise NotImplementedError
    async def connect_write_pipe(self, protocol_factory, pipe): raise NotImplementedError
    async def subprocess_shell(self, *args, **kwargs): raise NotImplementedError
    async def subprocess_exec(self, *args, **kwargs): raise NotImplementedError
    def add_reader(self, fd, callback, *args): raise NotImplementedError
    def remove_reader(self, fd): raise NotImplementedError
    def add_writer(self, fd, callback, *args): raise NotImplementedError
    def remove_writer(self, fd): raise NotImplementedError
    async def sock_recv(self, sock, nbytes): raise NotImplementedError
    async def sock_recv_into(self, sock, buf): raise NotImplementedError
    async def sock_recvfrom(self, sock, bufsize): raise NotImplementedError
    async def sock_recvfrom_into(self, sock, buf, nbytes=0): raise NotImplementedError
    async def sock_sendall(self, sock, data): raise NotImplementedError
    async def sock_sendto(self, sock, data, address): raise NotImplementedError
    async def sock_connect(self, sock, address): raise NotImplementedError
    async def sock_accept(self, sock): raise NotImplementedError
    async def sock_sendfile(self, sock, file, offset=0, count=None, *, fallback=None): raise NotImplementedError
    def add_signal_handler(self, sig, callback, *args): raise NotImplementedError
    def remove_signal_handler(self, sig): raise NotImplementedError
    def set_task_factory(self, factory): raise NotImplementedError
    def get_task_factory(self): raise NotImplementedError
    def get_exception_handler(self): raise NotImplementedError
    def set_exception_handler(self, handler): raise NotImplementedError
    def default_exception_handler(self, context): raise NotImplementedError
    def set_debug(self, enabled): raise NotImplementedError
    async def shutdown_default_executor(self): raise NotImplementedError
    def _timer_handle_cancelled(self, handle): raise NotImplementedError

async def register(loop: PollLoop, pollable):
    """Register a pollable with the event loop."""
    waker = loop.create_future()
    loop.wakers.append((pollable, waker))
    await waker

# Copy Spin SDK's send function exactly
async def send(request) -> Any:
    """Send the specified request and wait asynchronously for the response."""
    future = outgoing_handler.handle(request, None)
    
    while True:
        response = future.get()
        if response is None:
            await register(asyncio.get_event_loop(), future.subscribe())
        else:
            future.__exit__(None, None, None)
            
            if isinstance(response, Ok):
                if isinstance(response.value, Ok):
                    return response.value.value
                else:
                    raise response.value
            else:
                raise response

# Copy Spin SDK's Stream class
class Stream:
    """Reader abstraction over wasi:http/types#incoming-body."""
    def __init__(self, body):
        self.body = body
        self.stream = body.stream()
    
    async def next(self) -> Optional[bytes]:
        """Wait for the next chunk of data to arrive on the stream."""
        while True:
            try:
                if self.stream is None:
                    return None
                else:
                    buffer = self.stream.read(16 * 1024)
                    if len(buffer) == 0:
                        await register(asyncio.get_event_loop(), self.stream.subscribe())
                    else:
                        return buffer
            except Err as e:
                if isinstance(e.value, StreamError_Closed):
                    if self.stream is not None:
                        self.stream.__exit__(None, None, None)
                        self.stream = None
                    if self.body is not None:
                        IncomingBody.finish(self.body)
                        self.body = None
                else:
                    raise e


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
        
        # Execute weather fetches CONCURRENTLY using asyncio!
        loop = PollLoop()
        asyncio.set_event_loop(loop)
        
        async def fetch_all():
            """Fetch weather for all cities concurrently."""
            tasks = []
            for city in cities:
                tasks.append(self._fetch_weather_with_error_handling(city))
            return await asyncio.gather(*tasks)
        
        results = loop.run_until_complete(fetch_all())
        
        output = "=== Concurrent Weather Results ===\n\n"
        for result in results:
            output += f"{result}\n\n"
        output += "=== All requests completed ==="
        
        return output
    
    async def _fetch_weather_with_error_handling(self, city: str) -> str:
        """Fetch weather for a city with error handling."""
        try:
            # Get weather data
            geocoding_url = f"https://geocoding-api.open-meteo.com/v1/search?name={urllib.parse.quote(city)}&count=1"
            geo_data = await self._send_http_request_async(geocoding_url)
            
            if not geo_data.get("results"):
                return f"Error fetching weather for {city}: Location not found"
            
            location_data = geo_data["results"][0]
            
            weather_url = (
                f"https://api.open-meteo.com/v1/forecast?"
                f"latitude={location_data['latitude']}&longitude={location_data['longitude']}"
                f"&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code"
            )
            
            weather_data = await self._send_http_request_async(weather_url)
            
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
            return f"Error fetching weather for {city}: {str(e)}"
    
    async def _send_http_request_async(self, url: str) -> Dict[str, Any]:
        """Async HTTP GET request using WASI HTTP, copying Spin SDK exactly."""
        # Parse URL (copying Spin SDK)
        parsed = urlparse(url)
        
        # Determine scheme (copying Spin SDK)
        if parsed.scheme == 'https':
            scheme = http_types.Scheme_Https()
        elif parsed.scheme == 'http':
            scheme = http_types.Scheme_Http()
        else:
            scheme = http_types.Scheme_Http()  # Default to HTTP
        
        # Set authority (copying Spin SDK)
        if parsed.netloc == '':
            if isinstance(scheme, http_types.Scheme_Http):
                authority = ":80"
            else:
                authority = ":443"
        else:
            authority = parsed.netloc
        
        # Set path and query (copying Spin SDK)
        path_and_query = parsed.path if parsed.path else "/"
        if parsed.query:
            path_and_query += '?' + parsed.query
        
        # Create headers
        headers = http_types.Fields.from_list([
            ("accept", b"application/json"),
        ])
        
        # Create request (copying Spin SDK)
        outgoing_request = http_types.OutgoingRequest(headers)
        outgoing_request.set_method(http_types.Method_Get())
        outgoing_request.set_scheme(scheme)
        outgoing_request.set_authority(authority)
        outgoing_request.set_path_with_query(path_and_query)
        
        # Send request using Spin SDK's async send
        incoming_response = await send(outgoing_request)
        
        # Read response body using Stream class
        response_body = Stream(incoming_response.consume())
        body = bytearray()
        while True:
            chunk = await response_body.next()
            if chunk is None:
                break
            else:
                body += chunk
        
        # Clean up response
        incoming_response.__exit__(None, None, None)
        
        # Parse and return JSON
        return json.loads(bytes(body).decode('utf-8'))
    
    def _send_http_request(self, url: str) -> Dict[str, Any]:
        """Synchronous wrapper using Spin SDK's event loop."""
        loop = PollLoop()
        asyncio.set_event_loop(loop)
        return loop.run_until_complete(self._send_http_request_async(url))
    
    def _get_weather_for_city_sync(self, location: str) -> str:
        """Synchronous version of weather fetching using WASI HTTP."""
        try:
            # Geocode the location
            geocoding_url = f"https://geocoding-api.open-meteo.com/v1/search?name={urllib.parse.quote(location)}&count=1"
            
            geo_data = self._send_http_request(geocoding_url)
            
            if not geo_data.get("results"):
                raise Exception(f"Location '{location}' not found")
            
            location_data = geo_data["results"][0]
            
            # Fetch the weather
            weather_url = (
                f"https://api.open-meteo.com/v1/forecast?"
                f"latitude={location_data['latitude']}&longitude={location_data['longitude']}"
                f"&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code"
            )
            
            weather_data = self._send_http_request(weather_url)
            
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