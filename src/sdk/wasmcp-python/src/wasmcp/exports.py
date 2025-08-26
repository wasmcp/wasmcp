"""WIT exports for wasmcp Python SDK."""

import json
from typing import TYPE_CHECKING, Any, Dict, List, Optional

from .response import McpResponse

if TYPE_CHECKING:
    from .handler import WasmcpHandler

# Global exports instance
_exports: Optional["WasmcpExports"] = None


class WasmcpExports:
    """Handles WIT exports for MCP protocol."""
    
    def __init__(self, handler: "WasmcpHandler"):
        """Initialize exports with handler.
        
        Args:
            handler: The WasmcpHandler instance
        """
        self.handler = handler
    
    def list_tools(self) -> List[Dict[str, Any]]:
        """List available tools.
        
        Returns:
            List of tool definitions
        """
        return [tool.to_dict() for tool in self.handler._tools.values()]
    
    def call_tool(self, name: str, arguments: str) -> Dict[str, Any]:
        """Call a tool by name.
        
        Args:
            name: Tool name
            arguments: JSON string of arguments
            
        Returns:
            Tool result or error
        """
        if name not in self.handler._tools:
            return McpResponse.method_not_found(f"tools/{name}")
        
        tool = self.handler._tools[name]
        return tool.call(arguments)
    
    def list_resources(self) -> List[Dict[str, Any]]:
        """List available resources.
        
        Returns:
            List of resource definitions
        """
        return [resource.to_dict() for resource in self.handler._resources.values()]
    
    def read_resource(self, uri: str) -> Dict[str, Any]:
        """Read a resource by URI.
        
        Args:
            uri: Resource URI
            
        Returns:
            Resource content or error
        """
        if uri not in self.handler._resources:
            return McpResponse.method_not_found(f"resources/read")
        
        resource = self.handler._resources[uri]
        return resource.read()
    
    def list_prompts(self) -> List[Dict[str, Any]]:
        """List available prompts.
        
        Returns:
            List of prompt definitions
        """
        return [prompt.to_dict() for prompt in self.handler._prompts.values()]
    
    def get_prompt(self, name: str, arguments: str) -> Dict[str, Any]:
        """Get a prompt by name.
        
        Args:
            name: Prompt name
            arguments: JSON string of arguments
            
        Returns:
            Prompt result or error
        """
        if name not in self.handler._prompts:
            return McpResponse.method_not_found(f"prompts/{name}")
        
        prompt = self.handler._prompts[name]
        return prompt.get_prompt(arguments)


# WIT export functions - these are called by the WASM runtime
def list_tools() -> str:
    """WIT export: List available tools."""
    if _exports is None:
        return json.dumps([])
    return json.dumps(_exports.list_tools())


def call_tool(name: str, arguments: str) -> str:
    """WIT export: Call a tool."""
    if _exports is None:
        return json.dumps(McpResponse.internal_error("Handler not initialized"))
    return json.dumps(_exports.call_tool(name, arguments))


def list_resources() -> str:
    """WIT export: List available resources."""
    if _exports is None:
        return json.dumps([])
    return json.dumps(_exports.list_resources())


def read_resource(uri: str) -> str:
    """WIT export: Read a resource."""
    if _exports is None:
        return json.dumps(McpResponse.internal_error("Handler not initialized"))
    return json.dumps(_exports.read_resource(uri))


def list_prompts() -> str:
    """WIT export: List available prompts."""
    if _exports is None:
        return json.dumps([])
    return json.dumps(_exports.list_prompts())


def get_prompt(name: str, arguments: str) -> str:
    """WIT export: Get a prompt."""
    if _exports is None:
        return json.dumps(McpResponse.internal_error("Handler not initialized"))
    return json.dumps(_exports.get_prompt(name, arguments))