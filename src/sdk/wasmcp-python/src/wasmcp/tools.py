"""Tool management for MCP handlers."""

import asyncio
import json
from typing import Any, Callable, Dict, List, Optional, Union
from .schema import generate_function_input_schema, validate_against_schema
from .response import McpResponse


class Tool:
    """Represents an MCP tool (function call capability)."""
    
    def __init__(
        self,
        func: Callable,
        name: Optional[str] = None,
        description: Optional[str] = None
    ):
        """Initialize a tool.
        
        Args:
            func: The function to wrap
            name: Tool name (defaults to function name)
            description: Tool description (defaults to function docstring)
        """
        self.func = func
        self.name = name or func.__name__
        self.description = description or (func.__doc__ or "").strip()
        self.input_schema = generate_function_input_schema(func)
    
    @classmethod
    def from_function(
        cls,
        func: Callable,
        name: Optional[str] = None,
        description: Optional[str] = None
    ) -> "Tool":
        """Create a Tool from a function.
        
        Args:
            func: Function to wrap
            name: Optional custom name
            description: Optional custom description
            
        Returns:
            Tool instance
        """
        return cls(func, name, description)
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert tool to MCP tool format.
        
        Returns:
            MCP tool dictionary
        """
        return {
            "name": self.name,
            "description": self.description,
            "inputSchema": self.input_schema
        }
    
    def call(self, args: Union[str, Dict[str, Any]]) -> Dict[str, Any]:
        """Call the tool with given arguments.
        
        Args:
            args: Arguments as JSON string or dict
            
        Returns:
            MCP response
        """
        try:
            # Parse arguments if string
            if isinstance(args, str):
                try:
                    parsed_args = json.loads(args) if args.strip() else {}
                except json.JSONDecodeError as e:
                    return McpResponse.invalid_params(f"Invalid JSON arguments: {e}")
            else:
                parsed_args = args or {}
            
            # Validate arguments against schema
            validation_error = validate_against_schema(parsed_args, self.input_schema)
            if validation_error:
                return McpResponse.invalid_params(f"Argument validation failed: {validation_error}")
            
            # Call the function
            if asyncio.iscoroutinefunction(self.func):
                # Handle async function
                result = asyncio.run(self.func(**parsed_args))
            else:
                result = self.func(**parsed_args)
            
            # Format result
            if isinstance(result, (str, int, float, bool)):
                return McpResponse.success({"text": str(result)})
            elif isinstance(result, (dict, list)):
                return McpResponse.success({"text": json.dumps(result, indent=2)})
            else:
                return McpResponse.success({"text": str(result)})
                
        except Exception as e:
            return McpResponse.internal_error(f"Tool execution failed: {str(e)}")