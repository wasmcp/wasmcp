"""WIT export implementation for MCP handler."""

import json
from typing import List
from .bindings.wit_world.exports.handler import (
    Tool, ResourceInfo, ResourceContents, Prompt, PromptArgument,
    PromptMessage, Error, ToolResult_Text, ToolResult_Error,
    ResourceResult_Contents, ResourceResult_Error,
    PromptResult_Messages, PromptResult_Error
)

# Global handler registry - will be populated when handler modules are imported
_handler_instance = None

def set_handler(handler):
    """Set the global handler instance."""
    global _handler_instance
    _handler_instance = handler

def list_tools() -> List[Tool]:
    """List all registered tools."""
    if not _handler_instance:
        return []
    
    tools = []
    for name, tool_func in _handler_instance.tools.items():
        # Get the input schema from the tool function
        schema = getattr(tool_func, '__input_schema__', '{}')
        description = getattr(tool_func, '__description__', '') or tool_func.__doc__ or ''
        tools.append(Tool(name=name, description=description, input_schema=schema))
    return tools

def call_tool(name: str, arguments: str):
    """Call a tool with JSON arguments."""
    if not _handler_instance or name not in _handler_instance.tools:
        error = Error(code=-32601, message=f"Tool '{name}' not found", data=None)
        return ToolResult_Error(error)
    
    tool_func = _handler_instance.tools[name]
    try:
        args = json.loads(arguments) if arguments else {}
        result = tool_func(**args)
        # Convert result to JSON string
        if not isinstance(result, str):
            result = json.dumps(result)
        return ToolResult_Text(result)
    except json.JSONDecodeError as e:
        error = Error(code=-32700, message=f"Invalid JSON: {str(e)}", data=None)
        return ToolResult_Error(error)
    except Exception as e:
        error = Error(code=-32603, message=str(e), data=None)
        return ToolResult_Error(error)

def list_resources() -> List[ResourceInfo]:
    """List all registered resources."""
    if not _handler_instance:
        return []
    
    resources = []
    for uri, resource_func in _handler_instance.resources.items():
        description = getattr(resource_func, '__description__', '') or resource_func.__doc__ or ''
        name = getattr(resource_func, '__resource_name__', uri)
        mime_type = getattr(resource_func, '__mime_type__', None)
        resources.append(ResourceInfo(
            uri=uri, 
            name=name, 
            description=description, 
            mime_type=mime_type
        ))
    return resources

def read_resource(uri: str):
    """Read a resource by URI."""
    if not _handler_instance or uri not in _handler_instance.resources:
        error = Error(code=-32601, message=f"Resource '{uri}' not found", data=None)
        return ResourceResult_Error(error)
    
    resource_func = _handler_instance.resources[uri]
    try:
        content = resource_func()
        # Convert content to string if necessary
        if not isinstance(content, str):
            content = json.dumps(content)
        
        mime_type = getattr(resource_func, '__mime_type__', 'application/json')
        return ResourceResult_Contents(ResourceContents(
            uri=uri,
            mime_type=mime_type,
            text=content,
            blob=None
        ))
    except Exception as e:
        error = Error(code=-32603, message=str(e), data=None)
        return ResourceResult_Error(error)

def list_prompts() -> List[Prompt]:
    """List all registered prompts."""
    if not _handler_instance:
        return []
    
    prompts = []
    for name, prompt_func in _handler_instance.prompts.items():
        description = getattr(prompt_func, '__description__', '') or prompt_func.__doc__ or ''
        arguments = getattr(prompt_func, '__prompt_arguments__', [])
        
        prompt_args = []
        for arg in arguments:
            prompt_args.append(PromptArgument(
                name=arg['name'],
                description=arg.get('description'),
                required=arg.get('required', False)
            ))
        
        prompts.append(Prompt(
            name=name,
            description=description,
            arguments=prompt_args
        ))
    return prompts

def get_prompt(name: str, arguments: str):
    """Get a prompt with arguments."""
    if not _handler_instance or name not in _handler_instance.prompts:
        error = Error(code=-32601, message=f"Prompt '{name}' not found", data=None)
        return PromptResult_Error(error)
    
    prompt_func = _handler_instance.prompts[name]
    try:
        args = json.loads(arguments) if arguments else {}
        messages = prompt_func(**args)
        
        # Convert to PromptMessage objects
        prompt_messages = []
        for msg in messages:
            if isinstance(msg, dict):
                prompt_messages.append(PromptMessage(
                    role=msg.get('role', 'user'),
                    content=msg.get('content', '')
                ))
            else:
                # Handle simple string messages
                prompt_messages.append(PromptMessage(role='user', content=str(msg)))
        
        return PromptResult_Messages(prompt_messages)
    except json.JSONDecodeError as e:
        error = Error(code=-32700, message=f"Invalid JSON: {str(e)}", data=None)
        return PromptResult_Error(error)
    except Exception as e:
        error = Error(code=-32603, message=str(e), data=None)
        return PromptResult_Error(error)