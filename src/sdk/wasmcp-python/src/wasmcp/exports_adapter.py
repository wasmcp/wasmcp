"""Adapter to expose Handler class for componentize-py.

This module provides the Handler class that componentize-py expects,
wrapping our functional exports in a class interface.
"""

from .wit.exports import (
    list_tools as _list_tools,
    call_tool as _call_tool,
    list_resources as _list_resources,
    read_resource as _read_resource,
    list_prompts as _list_prompts,
    get_prompt as _get_prompt,
    set_handler
)


class Handler:
    """Handler class that componentize-py expects to find."""
    
    def list_tools(self):
        """List all registered tools."""
        return _list_tools()
    
    def call_tool(self, name: str, arguments: str):
        """Call a tool with JSON arguments."""
        return _call_tool(name, arguments)
    
    def list_resources(self):
        """List all registered resources."""
        return _list_resources()
    
    def read_resource(self, uri: str):
        """Read a resource by URI."""
        return _read_resource(uri)
    
    def list_prompts(self):
        """List all registered prompts."""
        return _list_prompts()
    
    def get_prompt(self, name: str, arguments: str):
        """Get a prompt with arguments."""
        return _get_prompt(name, arguments)


def setup_handler(user_handler):
    """Set up the user's handler instance for exports.
    
    Args:
        user_handler: Instance of WasmcpHandler with registered tools/resources/prompts
    """
    set_handler(user_handler)