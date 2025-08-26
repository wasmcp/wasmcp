"""Prompt management for MCP handlers."""

import asyncio
import json
from typing import Any, Callable, Dict, List, Optional, Union
from .schema import generate_function_input_schema, validate_against_schema
from .response import McpResponse


class Prompt:
    """Represents an MCP prompt template."""
    
    def __init__(
        self,
        func: Callable,
        name: Optional[str] = None,
        description: Optional[str] = None
    ):
        """Initialize a prompt.
        
        Args:
            func: Function that generates the prompt
            name: Prompt name (defaults to function name)
            description: Prompt description (defaults to function docstring)
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
    ) -> "Prompt":
        """Create a Prompt from a function.
        
        Args:
            func: Function that generates prompt
            name: Optional custom name
            description: Optional custom description
            
        Returns:
            Prompt instance
        """
        return cls(func, name, description)
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert prompt to MCP prompt format.
        
        Returns:
            MCP prompt dictionary
        """
        result = {
            "name": self.name
        }
        
        if self.description:
            result["description"] = self.description
        
        # Add arguments from input schema
        if self.input_schema.get("properties"):
            arguments = []
            required_props = set(self.input_schema.get("required", []))
            
            for prop_name, prop_schema in self.input_schema["properties"].items():
                arg = {
                    "name": prop_name,
                    "required": prop_name in required_props
                }
                
                if "type" in prop_schema:
                    if isinstance(prop_schema["type"], list):
                        # Handle multiple types (e.g., Optional)
                        arg["type"] = prop_schema["type"][0]  # Use first non-null type
                    else:
                        arg["type"] = prop_schema["type"]
                
                arguments.append(arg)
            
            if arguments:
                result["arguments"] = arguments
        
        return result
    
    def get_prompt(self, args: Union[str, Dict[str, Any]]) -> Dict[str, Any]:
        """Generate the prompt with given arguments.
        
        Args:
            args: Arguments as JSON string or dict
            
        Returns:
            MCP prompt response
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
                messages = asyncio.run(self.func(**parsed_args))
            else:
                messages = self.func(**parsed_args)
            
            # Ensure messages is a list
            if not isinstance(messages, list):
                return McpResponse.internal_error("Prompt function must return a list of messages")
            
            # Convert messages to MCP format
            mcp_messages = []
            for msg in messages:
                if isinstance(msg, dict) and "role" in msg and "content" in msg:
                    mcp_msg = {
                        "role": msg["role"],
                        "content": {
                            "type": "text",
                            "text": msg["content"]
                        }
                    }
                    mcp_messages.append(mcp_msg)
                else:
                    return McpResponse.internal_error("Invalid message format in prompt")
            
            return McpResponse.success({
                "description": self.description,
                "messages": mcp_messages
            })
            
        except Exception as e:
            return McpResponse.internal_error(f"Prompt generation failed: {str(e)}")