"""Main handler class for wasmcp Python SDK."""

from typing import Any, Callable, Dict, Optional

from .tools import Tool
from .resources import Resource
from .prompts import Prompt


class WasmcpHandler:
    """Main handler for wasmcp MCP components.
    
    This class provides a decorator-based API for registering tools,
    resources, and prompts that will be exposed via the WebAssembly
    Component Model interface.
    """
    
    def __init__(self, name: str = "wasmcp-python-handler"):
        """Initialize a wasmcp handler.
        
        Args:
            name: Handler name for identification
        """
        self.name = name
        self._tools: Dict[str, Tool] = {}
        self._resources: Dict[str, Resource] = {}
        self._prompts: Dict[str, Prompt] = {}
        
        # Expose as public attributes for exports.py compatibility
        self.tools = self._tools
        self.resources = self._resources
        self.prompts = self._prompts
        
        # Auto-register this handler for WIT bindings
        from .wit_impl import register_handler
        register_handler(self)
    
    @property
    def tool(self):
        """Decorator for registering tools.
        
        Usage:
            @handler.tool
            def my_tool(param: str) -> str:
                return f"Result: {param}"
        
        Or with options:
            @handler.tool(name="custom_name", description="Custom description")
            def my_tool(param: str) -> str:
                return f"Result: {param}"
        """
        def decorator(func_or_options=None, **kwargs):
            if func_or_options is None:
                # Called with keyword arguments: @handler.tool(name="...")
                def inner_decorator(func: Callable) -> Callable:
                    tool = Tool.from_function(func, **kwargs)
                    self._tools[tool.name] = tool
                    return func
                return inner_decorator
            elif callable(func_or_options):
                # Direct decoration: @handler.tool
                tool = Tool.from_function(func_or_options, **kwargs)
                self._tools[tool.name] = tool
                return func_or_options
            else:
                # Called with positional args (shouldn't happen but handle gracefully)
                raise TypeError("Invalid arguments to tool decorator")
        
        return decorator
    
    @property
    def resource(self):
        """Decorator for registering resources.
        
        Usage:
            @handler.resource(uri="config://settings")
            def get_settings() -> dict:
                return {"version": "1.0.0"}
        
        With options:
            @handler.resource(
                uri="data://users",
                name="User Data",
                mime_type="application/json"
            )
            def get_users() -> list:
                return [{"id": 1, "name": "Alice"}]
        """
        def decorator(**options):
            def inner(func: Callable) -> Callable:
                if "uri" not in options:
                    raise ValueError("Resource decorator requires 'uri' parameter")
                
                resource = Resource.from_function(func, **options)
                self._resources[resource.uri] = resource
                return func
            return inner
        
        return decorator
    
    @property
    def prompt(self):
        """Decorator for registering prompts.
        
        Usage:
            @handler.prompt
            def code_review() -> list:
                return [
                    {"role": "system", "content": "You are a code reviewer."},
                    {"role": "user", "content": "Review this code: {{code}}"}
                ]
        
        With options:
            @handler.prompt(name="custom_prompt", description="Custom prompt")
            def my_prompt(language: str = "python") -> list:
                return [
                    {"role": "system", "content": f"Review {language} code"}
                ]
        """
        def decorator(func_or_options=None, **kwargs):
            if func_or_options is None:
                # Called with keyword arguments: @handler.prompt(name="...")
                def inner_decorator(func: Callable) -> Callable:
                    prompt = Prompt.from_function(func, **kwargs)
                    self._prompts[prompt.name] = prompt
                    return func
                return inner_decorator
            elif callable(func_or_options):
                # Direct decoration: @handler.prompt
                prompt = Prompt.from_function(func_or_options, **kwargs)
                self._prompts[prompt.name] = prompt
                return func_or_options
            else:
                # Called with positional args (shouldn't happen but handle gracefully)
                raise TypeError("Invalid arguments to prompt decorator")
        
        return decorator
    
    def build(self) -> "WasmcpHandler":
        """Build and return the handler for export.
        
        This method ensures the handler is ready for use
        in the WASM component.
        
        Returns:
            The handler instance
        """
        return self
    
    def __repr__(self) -> str:
        """String representation of the handler."""
        return (
            f"WasmcpHandler(name='{self.name}', "
            f"tools={len(self._tools)}, "
            f"resources={len(self._resources)}, "
            f"prompts={len(self._prompts)})"
        )

# Alias for compatibility
Handler = WasmcpHandler