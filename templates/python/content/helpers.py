"""
Helper library for building MCP tool handlers with WebAssembly components.
Provides a decorator-based API for defining tools.
"""

import json
import asyncio
import inspect
import urllib.parse
from typing import Any, Callable, Dict, List, Optional, Union, TypeVar, get_type_hints
from functools import wraps
from dataclasses import dataclass, field

# Import componentize-py's built-in async HTTP support
# These are only available when running as Wasm
try:
    import poll_loop
    from poll_loop import PollLoop, Stream
except ImportError:
    # Not running in Wasm environment
    poll_loop = None
    PollLoop = None
    Stream = None

# Import MCP types from generated bindings
from wit_world.exports import ToolsCapabilities, CoreCapabilities
from wit_world.imports import (
    tools, 
    session_types,
    authorization_types,
    fastertools_mcp_types as mcp_types
)
from wit_world.imports.types import (
    OutgoingRequest,
    Fields,
    Scheme_Http,
    Scheme_Https,
    Method_Get,
)

T = TypeVar('T')


@dataclass
class Tool:
    """Represents a tool with its metadata and execution function."""
    name: str
    description: Optional[str] = None
    execute: Optional[Callable] = None
    schema: Optional[Dict[str, Any]] = None
    _original_fn: Optional[Callable] = None


@dataclass
class MCPServer:
    """
    Main MCP server class for WebAssembly components.
    Provides a decorator-based interface for defining tools.
    """
    name: str = "MCP Server"
    version: str = "0.1.0"
    instructions: Optional[str] = None
    tools: Dict[str, Tool] = field(default_factory=dict)
    auth_config: Optional[authorization_types.ProviderAuthConfig] = None
    protocol_version: session_types.ProtocolVersion = session_types.ProtocolVersion.V20250618
    
    def tool(
        self,
        name_or_fn: Optional[Union[str, Callable]] = None,
        *,
        name: Optional[str] = None,
        description: Optional[str] = None,
    ) -> Callable:
        """
        Decorator for registering tools with the MCP server.
        
        Can be used in multiple ways:
        - @server.tool (without parentheses)
        - @server.tool() (with empty parentheses)  
        - @server.tool("custom_name") (with name as first argument)
        - @server.tool(name="custom_name") (with name as keyword argument)
        
        Args:
            name_or_fn: Either a function (when used as @tool), a string name, or None
            name: Optional name for the tool (keyword-only, alternative to name_or_fn)
            description: Optional description for the tool
            
        Returns:
            Decorator function or decorated function
        """
        # Handle different decorator usage patterns
        if callable(name_or_fn):
            # Case: @server.tool (without parentheses)
            fn = name_or_fn
            tool_name = name or fn.__name__
            return self._register_tool(fn, tool_name, description)
        else:
            # Cases: @server.tool(), @server.tool("name"), @server.tool(name="name")
            if isinstance(name_or_fn, str):
                tool_name = name_or_fn
            else:
                tool_name = name
                
            def decorator(fn: Callable) -> Callable:
                actual_name = tool_name or fn.__name__
                return self._register_tool(fn, actual_name, description)
            return decorator
    
    def _register_tool(
        self,
        fn: Callable,
        name: str,
        description: Optional[str] = None
    ) -> Callable:
        """Internal method to register a tool function."""
        # Extract description from docstring if not provided
        if description is None:
            description = inspect.getdoc(fn)
        
        # Generate JSON schema from function signature
        schema = self._generate_schema(fn)
        
        # Create and store the tool
        tool = Tool(
            name=name,
            description=description,
            execute=fn,
            schema=schema,
            _original_fn=fn
        )
        self.tools[name] = tool
        
        # Return the original function unchanged
        return fn
    
    def _generate_schema(self, fn: Callable) -> Dict[str, Any]:
        """Generate JSON schema from function signature."""
        sig = inspect.signature(fn)
        type_hints = get_type_hints(fn)
        
        properties = {}
        required = []
        
        for param_name, param in sig.parameters.items():
            if param_name == 'self':
                continue
                
            # Get type hint if available
            param_type = type_hints.get(param_name, Any)
            
            # Convert Python types to JSON schema types
            json_type = self._python_type_to_json_schema(param_type)
            
            properties[param_name] = {
                "type": json_type,
                "description": f"Parameter {param_name}"
            }
            
            # Mark as required if no default value
            if param.default == inspect.Parameter.empty:
                required.append(param_name)
        
        return {
            "type": "object",
            "properties": properties,
            "required": required
        }
    
    def _python_type_to_json_schema(self, python_type) -> str:
        """Convert Python type to JSON schema type string."""
        if python_type == str:
            return "string"
        elif python_type == int:
            return "integer"
        elif python_type == float:
            return "number"
        elif python_type == bool:
            return "boolean"
        elif python_type == list or (hasattr(python_type, '__origin__') and python_type.__origin__ == list):
            return "array"
        elif python_type == dict or (hasattr(python_type, '__origin__') and python_type.__origin__ == dict):
            return "object"
        else:
            return "string"  # Default to string for unknown types
    
    def get_capabilities_handler(self) -> ToolsCapabilities:
        """
        Get the ToolsCapabilities handler for the WebAssembly component.
        
        Returns:
            A ToolsCapabilities instance that implements the WIT interface
        """
        return MCPCapabilities(self)
    
    def get_capabilities_class(self):
        """
        Get the combined capabilities class for the WebAssembly component.
        componentize-py expects a class, not an instance.
        
        Returns:
            A capabilities class that implements both ToolsCapabilities and CoreCapabilities
        """
        # Create a class that captures the server instance
        server = self
        
        class BoundMCPCapabilities(ToolsCapabilities, CoreCapabilities):
            # Tools capabilities
            def handle_list_tools(self, request):
                return MCPCapabilities(server).handle_list_tools(request)
            
            def handle_call_tool(self, request):
                return MCPCapabilities(server).handle_call_tool(request)
            
            # Core capabilities
            def handle_initialize(self, request):
                return MCPCapabilities(server).handle_initialize(request)
            
            def handle_initialized(self):
                return None
            
            def handle_ping(self):
                return None
            
            def handle_shutdown(self):
                return None
            
            def get_auth_config(self):
                return server.auth_config
        
        return BoundMCPCapabilities


class MCPCapabilities(ToolsCapabilities, CoreCapabilities):
    """MCP Capabilities implementation for WebAssembly components."""
    
    def __init__(self, server: MCPServer):
        self.server = server
    
    def handle_initialize(self, request: session_types.InitializeRequest) -> session_types.InitializeResponse:
        """Handle MCP initialization."""
        return session_types.InitializeResponse(
            protocol_version=self.server.protocol_version,
            capabilities=session_types.ServerCapabilities(
                experimental=None,
                logging=None,
                completions=None,
                prompts=None,
                resources=None,
                tools=session_types.ToolsCapability(
                    list_changed=None
                )
            ),
            server_info=session_types.ImplementationInfo(
                name=self.server.name,
                version=self.server.version,
                title=self.server.instructions
            ),
            instructions=self.server.instructions,
            meta=None
        )
    
    def handle_initialized(self) -> None:
        """Handle post-initialization."""
        return None
    
    def handle_ping(self) -> None:
        """Handle ping request."""
        return None
    
    def handle_shutdown(self) -> None:
        """Handle shutdown request."""
        return None
    
    def get_auth_config(self):
        """Get auth configuration from server."""
        return self.server.auth_config
    
    def handle_list_tools(self, request: tools.ListToolsRequest) -> tools.ListToolsResponse:
        """List available tools."""
        tool_list = []
        
        for tool in self.server.tools.values():
            tool_list.append(tools.Tool(
                base=mcp_types.BaseMetadata(
                    name=tool.name,
                    title=tool.name
                ),
                description=tool.description,
                input_schema=json.dumps(tool.schema),
                output_schema=None,
                annotations=None,
                meta=None
            ))
        
        return tools.ListToolsResponse(
            tools=tool_list,
            next_cursor=None,
            meta=None
        )
    
    def handle_call_tool(self, request: tools.CallToolRequest) -> tools.ToolResult:
        """Execute a tool."""
        tool = self.server.tools.get(request.name)
        
        if not tool or not tool.execute:
            return error_result(f"Unknown tool: {request.name}")
        
        try:
            # Parse arguments
            args = {}
            if request.arguments:
                args = json.loads(request.arguments)
            
            # Handle async functions
            if inspect.iscoroutinefunction(tool.execute):
                # Use componentize-py's PollLoop for async execution
                loop = PollLoop()
                asyncio.set_event_loop(loop)
                try:
                    result = loop.run_until_complete(tool.execute(**args))
                finally:
                    loop.close()
            else:
                # Synchronous function
                result = tool.execute(**args)
            
            # Convert result to MCP format
            if isinstance(result, tools.ToolResult):
                return result
            else:
                return text_result(str(result))
                
        except Exception as e:
            return error_result(f"Tool execution failed: {str(e)}")


# Helper functions for creating MCP results

def text_result(text: str) -> tools.ToolResult:
    """Create a text result in MCP format."""
    return tools.ToolResult(
        content=[mcp_types.ContentBlock_Text(
            value=mcp_types.TextContent(
                text=text,
                annotations=None,
                meta=None
            )
        )],
        structured_content=None,
        is_error=False,
        meta=None
    )


def error_result(message: str) -> tools.ToolResult:
    """Create an error result in MCP format."""
    return tools.ToolResult(
        content=[mcp_types.ContentBlock_Text(
            value=mcp_types.TextContent(
                text=message,
                annotations=None,
                meta=None
            )
        )],
        structured_content=None,
        is_error=True,
        meta=None
    )


def json_result(data: Any, text: Optional[str] = None) -> tools.ToolResult:
    """Create a result with structured JSON content."""
    # If no text provided, use JSON representation
    if text is None:
        text = json.dumps(data, indent=2)
    
    return tools.ToolResult(
        content=[mcp_types.ContentBlock_Text(
            value=mcp_types.TextContent(
                text=text,
                annotations=None,
                meta=None
            )
        )],
        structured_content=json.dumps(data),
        is_error=False,
        meta=None
    )


# HTTP helper functions

async def fetch_json(url: str) -> dict:
    """
    Fetch JSON from a URL using componentize-py's built-in HTTP support.
    
    Args:
        url: The URL to fetch
        
    Returns:
        Parsed JSON response as a dictionary
    """
    # Parse URL
    parsed = urllib.parse.urlparse(url)
    
    # Create request
    request = OutgoingRequest(Fields.from_list([]))
    
    # Set scheme
    if parsed.scheme == "https":
        request.set_scheme(Scheme_Https())
    else:
        request.set_scheme(Scheme_Http())
    
    # Set authority (host:port)
    request.set_authority(parsed.netloc)
    
    # Set path and query
    path_with_query = parsed.path
    if parsed.query:
        path_with_query += f"?{parsed.query}"
    request.set_path_with_query(path_with_query)
    
    # Set method
    request.set_method(Method_Get())
    
    # Send request using componentize-py's poll_loop.send()
    response = await poll_loop.send(request)
    
    # Check status
    status = response.status()
    if status < 200 or status >= 300:
        raise Exception(f"HTTP {status}")
    
    # Read body
    stream = Stream(response.consume())
    chunks = []
    while True:
        chunk = await stream.next()
        if chunk is None:
            break
        chunks.append(chunk)
    
    # Parse JSON
    body = b"".join(chunks)
    return json.loads(body)


def with_async_support(fn: Callable) -> Callable:
    """
    Decorator to handle both sync and async functions uniformly.
    Useful for tool functions that might need async HTTP calls.
    """
    if inspect.iscoroutinefunction(fn):
        return fn
    else:
        @wraps(fn)
        async def async_wrapper(*args, **kwargs):
            return fn(*args, **kwargs)
        return async_wrapper