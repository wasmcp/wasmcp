"""Resource management for MCP handlers."""

import asyncio
import json
from typing import Any, Callable, Dict, Optional, Union
from .response import McpResponse


class Resource:
    """Represents an MCP resource."""
    
    def __init__(
        self,
        func: Callable,
        uri: str,
        name: Optional[str] = None,
        description: Optional[str] = None,
        mime_type: str = "text/plain"
    ):
        """Initialize a resource.
        
        Args:
            func: Function that provides the resource data
            uri: Resource URI
            name: Resource name (defaults to function name)
            description: Resource description (defaults to function docstring)
            mime_type: MIME type of the resource
        """
        self.func = func
        self.uri = uri
        self.name = name or func.__name__
        self.description = description or (func.__doc__ or "").strip()
        self.mime_type = mime_type
    
    @classmethod
    def from_function(
        cls,
        func: Callable,
        uri: str,
        name: Optional[str] = None,
        description: Optional[str] = None,
        mime_type: str = "text/plain"
    ) -> "Resource":
        """Create a Resource from a function.
        
        Args:
            func: Function that provides resource data
            uri: Resource URI
            name: Optional custom name
            description: Optional custom description
            mime_type: MIME type
            
        Returns:
            Resource instance
        """
        return cls(func, uri, name, description, mime_type)
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert resource to MCP resource format.
        
        Returns:
            MCP resource dictionary
        """
        result = {
            "uri": self.uri,
            "name": self.name
        }
        
        if self.description:
            result["description"] = self.description
            
        if self.mime_type:
            result["mimeType"] = self.mime_type
            
        return result
    
    def read(self) -> Dict[str, Any]:
        """Read the resource data.
        
        Returns:
            MCP read response
        """
        try:
            # Call the function
            if asyncio.iscoroutinefunction(self.func):
                # Handle async function
                result = asyncio.run(self.func())
            else:
                result = self.func()
            
            # Format content based on type and MIME type
            if self.mime_type == "application/json":
                if isinstance(result, (dict, list)):
                    text_content = json.dumps(result, indent=2)
                else:
                    text_content = json.dumps(result)
            else:
                if isinstance(result, str):
                    text_content = result
                elif isinstance(result, (dict, list)):
                    text_content = json.dumps(result, indent=2)
                else:
                    text_content = str(result)
            
            return McpResponse.success({
                "contents": [
                    {
                        "uri": self.uri,
                        "mimeType": self.mime_type,
                        "text": text_content
                    }
                ]
            })
            
        except Exception as e:
            return McpResponse.internal_error(f"Resource read failed: {str(e)}")