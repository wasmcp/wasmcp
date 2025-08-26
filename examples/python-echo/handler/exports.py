"""WIT export implementation that bridges user handlers with componentize-py bindings.

This module implements the export functions that componentize-py expects,
using the bindings that componentize-py generates at build time.
"""

import json
from typing import List

# Import types from componentize-py generated bindings
# These are generated at build time into the local directory
from wit_world.exports.handler import (
    Tool, ResourceInfo, ResourceContents, Prompt, PromptArgument,
    PromptMessage, Error, ToolResult_Text, ToolResult_Error,
    ResourceResult_Contents, ResourceResult_Error,
    PromptResult_Messages, PromptResult_Error
)

# Import the user's handler
from src.app import handler

# Global reference to the user's handler instance
_handler_instance = handler


def list_tools() -> List[Tool]:
    """List all registered tools."""
    if not _handler_instance:
        return []
    
    tools = []
    for name, tool_obj in _handler_instance.tools.items():
        schema_json = json.dumps(tool_obj.input_schema) if hasattr(tool_obj, 'input_schema') else '{}'
        description = tool_obj.description if hasattr(tool_obj, 'description') else ''
        tools.append(Tool(name=name, description=description, input_schema=schema_json))
    return tools


def call_tool(name: str, arguments: str):
    """Call a tool with JSON arguments."""
    if not _handler_instance or name not in _handler_instance.tools:
        error = Error(code=-32601, message=f"Tool '{name}' not found", data=None)
        return ToolResult_Error(value=error)
    
    tool_obj = _handler_instance.tools[name]
    try:
        # Use the Tool object's call method
        result = tool_obj.call(arguments)
        
        # Extract content from MCP response
        if isinstance(result, dict):
            if 'result' in result:
                result_data = result['result']
                
                # Handle MCP content array format
                if 'content' in result_data and isinstance(result_data['content'], list):
                    content_item = result_data['content'][0] if result_data['content'] else {}
                    text = content_item.get('text', '')
                elif 'text' in result_data:
                    text = result_data.get('text', '')
                else:
                    text = json.dumps(result_data)
                    
                return ToolResult_Text(value=text)
            elif 'error' in result:
                error = Error(
                    code=result['error'].get('code', -32603),
                    message=result['error'].get('message', 'Unknown error'),
                    data=result['error'].get('data')
                )
                return ToolResult_Error(value=error)
        
        # Fallback: convert result to string
        return ToolResult_Text(value=str(result))
        
    except json.JSONDecodeError as e:
        error = Error(code=-32700, message=f"Invalid JSON: {str(e)}", data=None)
        return ToolResult_Error(value=error)
    except Exception as e:
        error = Error(code=-32603, message=str(e), data=None)
        return ToolResult_Error(value=error)


def list_resources() -> List[ResourceInfo]:
    """List all registered resources."""
    if not _handler_instance:
        return []
    
    resources = []
    for uri, resource_obj in _handler_instance.resources.items():
        description = resource_obj.description if hasattr(resource_obj, 'description') else ''
        name = resource_obj.name if hasattr(resource_obj, 'name') else uri
        mime_type = resource_obj.mime_type if hasattr(resource_obj, 'mime_type') else None
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
        return ResourceResult_Error(value=error)
    
    resource_obj = _handler_instance.resources[uri]
    try:
        # Use the Resource object's read method
        result = resource_obj.read()
        
        # Extract content from the result
        if isinstance(result, dict) and 'result' in result:
            contents = result['result'].get('contents', [])
            if contents and isinstance(contents, list):
                content = contents[0]
                mime_type = content.get('mime_type', 'text/plain')
                text = content.get('text', '')
            else:
                mime_type = 'text/plain'
                text = json.dumps(result['result'])
        else:
            mime_type = resource_obj.mime_type if hasattr(resource_obj, 'mime_type') else 'text/plain'
            text = str(result)
        
        return ResourceResult_Contents(value=ResourceContents(
            uri=uri,
            mime_type=mime_type,
            text=text,
            blob=None
        ))
    except Exception as e:
        error = Error(code=-32603, message=str(e), data=None)
        return ResourceResult_Error(value=error)


def list_prompts() -> List[Prompt]:
    """List all registered prompts."""
    if not _handler_instance:
        return []
    
    prompts = []
    for name, prompt_obj in _handler_instance.prompts.items():
        description = prompt_obj.description if hasattr(prompt_obj, 'description') else ''
        
        # Get arguments from the Prompt object's input_schema
        prompt_args = []
        if hasattr(prompt_obj, 'input_schema') and 'properties' in prompt_obj.input_schema:
            for arg_name, arg_schema in prompt_obj.input_schema['properties'].items():
                required = arg_name in prompt_obj.input_schema.get('required', [])
                prompt_args.append(PromptArgument(
                    name=arg_name,
                    description=arg_schema.get('description', ''),
                    required=required
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
        return PromptResult_Error(value=error)
    
    prompt_obj = _handler_instance.prompts[name]
    try:
        # Use the Prompt object's get_prompt method
        result = prompt_obj.get_prompt(arguments)
        
        # Extract messages from the MCP response
        if isinstance(result, dict):
            if 'result' in result:
                # Standard MCP response format
                messages_data = result['result'].get('messages', [])
            elif 'messages' in result:
                # Direct messages format
                messages_data = result['messages']
            else:
                messages_data = []
        else:
            messages_data = result if isinstance(result, list) else []
        
        # Convert to PromptMessage objects
        prompt_messages = []
        for msg in messages_data:
            if isinstance(msg, dict):
                # Handle nested content structure if present
                content = msg.get('content', '')
                if isinstance(content, dict) and 'text' in content:
                    content = content['text']
                
                prompt_messages.append(PromptMessage(
                    role=msg.get('role', 'user'),
                    content=str(content)
                ))
            else:
                prompt_messages.append(PromptMessage(role='user', content=str(msg)))
        
        return PromptResult_Messages(value=prompt_messages)
    except json.JSONDecodeError as e:
        error = Error(code=-32700, message=f"Invalid JSON: {str(e)}", data=None)
        return PromptResult_Error(value=error)
    except Exception as e:
        error = Error(code=-32603, message=str(e), data=None)
        return PromptResult_Error(value=error)