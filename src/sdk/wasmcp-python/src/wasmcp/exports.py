"""Exports adapter for componentize-py WIT bindings integration.

This module provides the bridge between the wasmcp SDK's high-level decorator API
and componentize-py's generated WIT bindings. It implements the MCP handler interface
that componentize-py expects to find.
"""

import json
from typing import List, Dict, Any, Optional

# Global registry for handler instances
_handler_registry = {
    "tools": {},
    "resources": {},
    "prompts": {}
}

def register_handler(handler_instance):
    """Register a handler instance for use by the exports interface.
    
    This is called automatically when a WasmcpHandler is created.
    """
    _handler_registry["tools"].update(handler_instance.tools)
    _handler_registry["resources"].update(handler_instance.resources)
    _handler_registry["prompts"].update(handler_instance.prompts)

# Static handler implementation that componentize-py can use
class StaticHandler:
    """Static handler that provides the MCP interface using registered handlers."""
    
    def list_tools(self):
        """List all registered tools."""
        tools = []
        for tool_name, tool_obj in _handler_registry["tools"].items():
            tools.append({
                "name": tool_obj.name,
                "description": tool_obj.description,
                "input_schema": tool_obj.input_schema
            })
        return tools
    
    def call_tool(self, name: str, arguments: str):
        """Call a registered tool by name."""
        try:
            if name not in _handler_registry["tools"]:
                return {"error": {"code": -32601, "message": f"Tool not found: {name}"}}
            
            tool_obj = _handler_registry["tools"][name]
            args_dict = json.loads(arguments) if arguments else {}
            
            result = tool_obj.func(**args_dict)
            return {"text": str(result)}
            
        except json.JSONDecodeError as e:
            return {"error": {"code": -32700, "message": f"Invalid JSON arguments: {str(e)}"}}
        except TypeError as e:
            return {"error": {"code": -32602, "message": f"Invalid parameters: {str(e)}"}}
        except Exception as e:
            return {"error": {"code": -32603, "message": f"Internal error: {str(e)}"}}
    
    def list_resources(self):
        """List all registered resources."""
        resources = []
        for uri, resource_obj in _handler_registry["resources"].items():
            resources.append({
                "uri": resource_obj.uri,
                "name": resource_obj.name or resource_obj.uri.split("/")[-1],
                "description": resource_obj.description,
                "mime_type": resource_obj.mime_type
            })
        return resources
    
    def read_resource(self, uri: str):
        """Read a registered resource by URI."""
        try:
            if uri not in _handler_registry["resources"]:
                return {"error": {"code": -32601, "message": f"Resource not found: {uri}"}}
            
            resource_obj = _handler_registry["resources"][uri]
            result = resource_obj.func()
            
            # Convert result to appropriate format
            if isinstance(result, (dict, list)):
                text_content = json.dumps(result, indent=2)
            else:
                text_content = str(result)
            
            return {
                "contents": {
                    "uri": uri,
                    "mime_type": resource_obj.mime_type,
                    "text": text_content,
                    "blob": None
                }
            }
            
        except Exception as e:
            return {"error": {"code": -32603, "message": f"Internal error: {str(e)}"}}
    
    def list_prompts(self):
        """List all registered prompts."""
        prompts = []
        for prompt_name, prompt_obj in _handler_registry["prompts"].items():
            # Extract arguments from function signature
            import inspect
            sig = inspect.signature(prompt_obj.func)
            arguments = []
            
            for param_name, param in sig.parameters.items():
                arg_info = {
                    "name": param_name,
                    "description": f"Parameter {param_name}",
                    "required": param.default == inspect.Parameter.empty
                }
                arguments.append(arg_info)
            
            prompts.append({
                "name": prompt_obj.name,
                "description": prompt_obj.description,
                "arguments": arguments
            })
        return prompts
    
    def get_prompt(self, name: str, arguments: str):
        """Get a prompt by name with arguments."""
        try:
            if name not in _handler_registry["prompts"]:
                return {"error": {"code": -32601, "message": f"Prompt not found: {name}"}}
            
            prompt_obj = _handler_registry["prompts"][name]
            args_dict = json.loads(arguments) if arguments else {}
            
            messages = prompt_obj.func(**args_dict)
            
            # Convert to expected format
            formatted_messages = []
            for msg in messages:
                if isinstance(msg, dict) and "role" in msg and "content" in msg:
                    formatted_messages.append({
                        "role": msg["role"],
                        "content": msg["content"]
                    })
                else:
                    # Handle other message formats
                    formatted_messages.append({
                        "role": "user",
                        "content": str(msg)
                    })
            
            return {"messages": formatted_messages}
            
        except json.JSONDecodeError as e:
            return {"error": {"code": -32700, "message": f"Invalid JSON arguments: {str(e)}"}}
        except Exception as e:
            return {"error": {"code": -32603, "message": f"Internal error: {str(e)}"}}

# Create the static handler instance
handler = StaticHandler()

# Export the interface functions that componentize-py expects
def list_tools():
    """Export function for componentize-py."""
    return handler.list_tools()

def call_tool(name: str, arguments: str):
    """Export function for componentize-py."""
    return handler.call_tool(name, arguments)

def list_resources():
    """Export function for componentize-py."""
    return handler.list_resources()

def read_resource(uri: str):
    """Export function for componentize-py."""
    return handler.read_resource(uri)

def list_prompts():
    """Export function for componentize-py."""
    return handler.list_prompts()

def get_prompt(name: str, arguments: str):
    """Export function for componentize-py."""
    return handler.get_prompt(name, arguments)