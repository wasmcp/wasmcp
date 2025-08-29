"""
Helper library for building MCP tool handlers in Python.

Provides Pythonic abstractions over the WIT interface, similar to the
JavaScript and Rust helper libraries.
"""

from typing import Any, Dict, List, Optional, Callable, Union, Protocol
from dataclasses import dataclass, field
import json
import inspect
import asyncio
from abc import ABC, abstractmethod


# Type aliases for clarity
JsonValue = Union[str, int, float, bool, None, Dict[str, Any], List[Any]]
JsonSchema = Dict[str, Any]


class Tool(ABC):
    """Base class for MCP tools."""
    
    @property
    @abstractmethod
    def name(self) -> str:
        """The tool's name."""
        pass
    
    @property
    @abstractmethod
    def description(self) -> str:
        """The tool's description."""
        pass
    
    @property
    def input_schema(self) -> JsonSchema:
        """The JSON schema for this tool's input."""
        return {"type": "object", "properties": {}}
    
    @abstractmethod
    async def execute(self, args: Dict[str, Any]) -> str:
        """Execute the tool with the given arguments."""
        pass


@dataclass
class ToolDefinition:
    """A tool definition with metadata and execution logic."""
    name: str
    description: str
    input_schema: JsonSchema
    execute_fn: Callable
    annotations: Optional[Dict[str, Any]] = None
    

def tool(
    name: str,
    description: str,
    schema: Optional[JsonSchema] = None,
    **annotations: Any
) -> Callable:
    """
    Decorator for creating tools in a Pythonic way.
    
    Example:
        @tool(
            name="echo",
            description="Echo a message back",
            schema={
                "type": "object",
                "properties": {
                    "message": {"type": "string"}
                },
                "required": ["message"]
            }
        )
        async def echo_tool(message: str) -> str:
            return f"Echo: {message}"
    """
    def decorator(func: Callable) -> ToolDefinition:
        # Extract parameter names from function signature
        sig = inspect.signature(func)
        
        # If no schema provided, try to infer from function signature
        if schema is None:
            properties = {}
            required = []
            
            for param_name, param in sig.parameters.items():
                if param_name not in ['self', 'cls']:
                    # Basic type inference
                    param_type = "string"  # default
                    if param.annotation != inspect.Parameter.empty:
                        if param.annotation == int:
                            param_type = "integer"
                        elif param.annotation == float:
                            param_type = "number"
                        elif param.annotation == bool:
                            param_type = "boolean"
                        elif param.annotation == list or param.annotation == List:
                            param_type = "array"
                        elif param.annotation == dict or param.annotation == Dict:
                            param_type = "object"
                    
                    properties[param_name] = {"type": param_type}
                    
                    if param.default == inspect.Parameter.empty:
                        required.append(param_name)
            
            inferred_schema = {
                "type": "object",
                "properties": properties,
                "required": required
            }
        else:
            inferred_schema = schema
        
        # Wrap the function to handle both sync and async
        if inspect.iscoroutinefunction(func):
            async def execute_wrapper(args: Dict[str, Any]) -> str:
                # Map arguments to function parameters
                kwargs = {}
                for param_name in sig.parameters:
                    if param_name in args:
                        kwargs[param_name] = args[param_name]
                
                result = await func(**kwargs)
                return str(result) if not isinstance(result, str) else result
        else:
            async def execute_wrapper(args: Dict[str, Any]) -> str:
                # Map arguments to function parameters
                kwargs = {}
                for param_name in sig.parameters:
                    if param_name in args:
                        kwargs[param_name] = args[param_name]
                
                result = func(**kwargs)
                return str(result) if not isinstance(result, str) else result
        
        return ToolDefinition(
            name=name,
            description=description,
            input_schema=inferred_schema,
            execute_fn=execute_wrapper,
            annotations=annotations
        )
    
    return decorator


def text_result(text: str) -> Dict[str, Any]:
    """Create a text result in MCP format."""
    return {
        "content": [{
            "tag": "text",
            "val": {
                "text": text,
                "annotations": None,
                "meta": None
            }
        }],
        "structuredContent": None,
        "isError": False,
        "meta": None
    }


def error_result(message: str) -> Dict[str, Any]:
    """Create an error result in MCP format."""
    return {
        "content": [{
            "tag": "text",
            "val": {
                "text": message,
                "annotations": None,
                "meta": None
            }
        }],
        "structuredContent": None,
        "isError": True,
        "meta": None
    }


class Handler:
    """
    MCP tool handler that manages a collection of tools.
    
    Example:
        handler = Handler()
        handler.register(echo_tool)
        handler.register(weather_tool)
        
        # Or use the decorator style:
        @handler.tool(name="echo", description="Echo a message")
        async def echo(message: str) -> str:
            return f"Echo: {message}"
    """
    
    def __init__(self):
        self.tools: Dict[str, ToolDefinition] = {}
    
    def register(self, tool_def: Union[Tool, ToolDefinition]) -> None:
        """Register a tool with the handler."""
        if isinstance(tool_def, Tool):
            # Convert Tool class to ToolDefinition
            self.tools[tool_def.name] = ToolDefinition(
                name=tool_def.name,
                description=tool_def.description,
                input_schema=tool_def.input_schema,
                execute_fn=tool_def.execute
            )
        elif isinstance(tool_def, ToolDefinition):
            self.tools[tool_def.name] = tool_def
        else:
            raise TypeError(f"Expected Tool or ToolDefinition, got {type(tool_def)}")
    
    def tool(
        self,
        name: str,
        description: str,
        schema: Optional[JsonSchema] = None,
        **annotations: Any
    ) -> Callable:
        """Decorator for registering tools directly on the handler."""
        def decorator(func: Callable) -> Callable:
            tool_def = tool(name, description, schema, **annotations)(func)
            self.register(tool_def)
            return func
        return decorator
    
    def handle_list_tools(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """Handle a list-tools request."""
        tool_definitions = []
        
        for tool_def in self.tools.values():
            definition = {
                "base": {
                    "name": tool_def.name,
                    "title": tool_def.name
                },
                "description": tool_def.description,
                "inputSchema": json.dumps(tool_def.input_schema),
                "outputSchema": None,
                "annotations": tool_def.annotations,
                "meta": None
            }
            tool_definitions.append(definition)
        
        return {
            "tools": tool_definitions,
            "nextCursor": None,
            "meta": None
        }
    
    async def handle_call_tool(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """Handle a call-tool request."""
        tool_name = request.get("name")
        
        if tool_name not in self.tools:
            return error_result(f"Unknown tool: {tool_name}")
        
        tool_def = self.tools[tool_name]
        
        try:
            # Parse arguments if they're a string
            args = request.get("arguments", {})
            if isinstance(args, str):
                args = json.loads(args) if args else {}
            
            # Execute the tool
            result = await tool_def.execute_fn(args)
            
            # If result is already in the correct format, return it
            if isinstance(result, dict) and "content" in result:
                return result
            
            # Otherwise, wrap it as text
            return text_result(result)
            
        except Exception as e:
            return error_result(f"Error executing {tool_name}: {str(e)}")


# Global handler instance for convenience
_default_handler = Handler()


def register_tool(tool_def: Union[Tool, ToolDefinition]) -> None:
    """Register a tool with the default handler."""
    _default_handler.register(tool_def)


def create_handler(tools: Optional[List[Union[Tool, ToolDefinition]]] = None) -> Handler:
    """Create a new handler with optional initial tools."""
    handler = Handler()
    if tools:
        for tool_def in tools:
            handler.register(tool_def)
    return handler


# Export functions for WIT interface compatibility
# These will be called by the generated bindings
def handle_list_tools(request):
    """Entry point for list-tools requests."""
    return _default_handler.handle_list_tools(request)


async def handle_call_tool(request):
    """Entry point for call-tool requests."""
    return await _default_handler.handle_call_tool(request)