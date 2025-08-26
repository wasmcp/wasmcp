"""Static WIT exports for componentize-py.

This module provides a static class structure that componentize-py can analyze
while still supporting the decorator-based API that users love.
"""

import json
from typing import List, Optional
from .wit.wit_world.exports import Handler as WitHandler
from .wit.wit_world.exports import handler as wit_types

# Global handler registry populated by decorator API
_handler_registry = {
    'tools': {},
    'resources': {},
    'prompts': {}
}

def register_handler(handler):
    """Register a handler instance for static access.
    
    This populates the global registry that the static MCPHandler uses.
    """
    global _handler_registry
    
    # Clear existing registry and populate from handler
    # Note: We replace rather than merge to support single-handler pattern
    if hasattr(handler, 'tools'):
        _handler_registry['tools'] = handler.tools.copy()
    if hasattr(handler, 'resources'):
        _handler_registry['resources'] = handler.resources.copy()
    if hasattr(handler, 'prompts'):
        _handler_registry['prompts'] = handler.prompts.copy()

class MCPHandler(WitHandler):
    """Static WIT exports implementation.
    
    This class provides a static implementation of the WIT Guest interface
    that componentize-py can analyze and build. It accesses the global
    registry populated by the decorator API.
    """
    
    def __init__(self):
        """Initialize the static handler.
        
        This constructor is simple and doesn't perform dynamic behavior
        that would confuse componentize-py's static analysis.
        """
        pass
    
    def list_tools(self) -> List[wit_types.Tool]:
        """Convert registered tools to WIT Tool format."""
        tools = []
        for tool in _handler_registry['tools'].values():
            tools.append(wit_types.Tool(
                name=tool.name,
                description=getattr(tool, 'description', '') or "",
                input_schema=json.dumps(tool.input_schema) if hasattr(tool, 'input_schema') and tool.input_schema else "{}"
            ))
        return tools
    
    def call_tool(self, name: str, arguments: str) -> wit_types.ToolResult:
        """Execute a tool by name with JSON args."""
        if name not in _handler_registry['tools']:
            error = wit_types.Error(
                code=-32601,
                message=f"Unknown tool: {name}",
                data=None
            )
            return wit_types.ToolResult_Error(error)
        
        tool = _handler_registry['tools'][name]
        try:
            parsed_args = json.loads(arguments) if arguments else {}
            
            # Validate against schema if tool has a validate method
            if hasattr(tool, 'validate'):
                validation_result = tool.validate(parsed_args)
                if validation_result is not None:
                    # Validation failed
                    error = wit_types.Error(
                        code=-32602,
                        message="Invalid parameters",
                        data=validation_result if isinstance(validation_result, str) else json.dumps(validation_result)
                    )
                    return wit_types.ToolResult_Error(error)
            
            # Execute the tool
            if hasattr(tool, 'execute'):
                result = tool.execute(parsed_args)
            elif hasattr(tool, 'func'):
                # Support simple function-based tools
                result = tool.func(**parsed_args)
            else:
                raise RuntimeError("Tool has no execute method or func")
            
            # In WASM, we can't use asyncio, so async functions need special handling
            if hasattr(result, '__await__'):
                # This won't work in WASM - return error
                error = wit_types.Error(
                    code=-32603,
                    message="Async functions not supported in WASM environment",
                    data=None
                )
                return wit_types.ToolResult_Error(error)
            
            # Convert result to string if needed
            if not isinstance(result, str):
                result = json.dumps(result)
            
            return wit_types.ToolResult_Text(result)
            
        except Exception as e:
            error = wit_types.Error(
                code=-32603,
                message=str(e),
                data=None
            )
            return wit_types.ToolResult_Error(error)
    
    def list_resources(self) -> List[wit_types.ResourceInfo]:
        """Convert registered resources to WIT ResourceInfo format."""
        resources = []
        for resource in _handler_registry['resources'].values():
            resources.append(wit_types.ResourceInfo(
                uri=getattr(resource, 'uri', ''),
                name=getattr(resource, 'name', ''),
                description=getattr(resource, 'description', None),
                mime_type=getattr(resource, 'mime_type', None)
            ))
        return resources
    
    def read_resource(self, uri: str) -> wit_types.ResourceResult:
        """Read a resource by URI."""
        if not _handler_registry['resources']:
            error = wit_types.Error(
                code=-32601,
                message="No resources available",
                data=None
            )
            return wit_types.ResourceResult_Error(error)
        
        # Find resource by URI
        resource = None
        for res in _handler_registry['resources'].values():
            if getattr(res, 'uri', '') == uri:
                resource = res
                break
        
        if not resource:
            error = wit_types.Error(
                code=-32601,
                message=f"Unknown resource: {uri}",
                data=None
            )
            return wit_types.ResourceResult_Error(error)
        
        try:
            # Execute the resource handler
            if hasattr(resource, 'execute'):
                content = resource.execute()
            elif hasattr(resource, 'read'):
                content = resource.read()
            else:
                raise RuntimeError("Resource has no execute or read method")
            
            # Handle async
            if hasattr(content, '__await__'):
                error = wit_types.Error(
                    code=-32603,
                    message="Async functions not supported in WASM environment",
                    data=None
                )
                return wit_types.ResourceResult_Error(error)
            
            # Create ResourceContents
            contents = wit_types.ResourceContents(
                uri=uri,
                mime_type=getattr(resource, 'mime_type', None),
                text=content if isinstance(content, str) else None,
                blob=content if isinstance(content, bytes) else None
            )
            
            return wit_types.ResourceResult_Contents(contents)
            
        except Exception as e:
            error = wit_types.Error(
                code=-32603,
                message=str(e),
                data=None
            )
            return wit_types.ResourceResult_Error(error)
    
    def list_prompts(self) -> List[wit_types.Prompt]:
        """Convert registered prompts to WIT Prompt format."""
        prompts = []
        for prompt in _handler_registry['prompts'].values():
            arguments = []
            if hasattr(prompt, 'arguments'):
                for arg_name, arg_info in prompt.arguments.items():
                    arguments.append(wit_types.PromptArgument(
                        name=arg_name,
                        description=arg_info.get('description') if isinstance(arg_info, dict) else None,
                        required=arg_info.get('required', False) if isinstance(arg_info, dict) else False
                    ))
            
            prompts.append(wit_types.Prompt(
                name=getattr(prompt, 'name', ''),
                description=getattr(prompt, 'description', None),
                arguments=arguments
            ))
        return prompts
    
    def get_prompt(self, name: str, arguments: str) -> wit_types.PromptResult:
        """Get a prompt by name with arguments."""
        if name not in _handler_registry['prompts']:
            error = wit_types.Error(
                code=-32601,
                message=f"Unknown prompt: {name}",
                data=None
            )
            return wit_types.PromptResult_Error(error)
        
        prompt = _handler_registry['prompts'][name]
        try:
            parsed_args = json.loads(arguments) if arguments else {}
            
            # Execute the prompt handler
            if hasattr(prompt, 'execute'):
                messages = prompt.execute(parsed_args)
            elif hasattr(prompt, 'get'):
                messages = prompt.get(parsed_args)
            else:
                raise RuntimeError("Prompt has no execute or get method")
            
            # Handle async
            if hasattr(messages, '__await__'):
                error = wit_types.Error(
                    code=-32603,
                    message="Async functions not supported in WASM environment",
                    data=None
                )
                return wit_types.PromptResult_Error(error)
            
            # Convert to PromptMessage list
            prompt_messages = []
            if isinstance(messages, list):
                for msg in messages:
                    if isinstance(msg, dict):
                        prompt_messages.append(wit_types.PromptMessage(
                            role=msg.get('role', 'user'),
                            content=msg.get('content', '')
                        ))
                    else:
                        # Simple string becomes user message
                        prompt_messages.append(wit_types.PromptMessage(
                            role='user',
                            content=str(msg)
                        ))
            else:
                # Single message
                prompt_messages.append(wit_types.PromptMessage(
                    role='user',
                    content=str(messages)
                ))
            
            return wit_types.PromptResult_Messages(prompt_messages)
            
        except Exception as e:
            error = wit_types.Error(
                code=-32603,
                message=str(e),
                data=None
            )
            return wit_types.PromptResult_Error(error)

# CRITICAL: This is what componentize-py looks for
# The variable name must be 'handler' and it must be a static instance
# No proxies, no lazy loading - just a simple static class instance
handler = MCPHandler()