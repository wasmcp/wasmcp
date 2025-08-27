"""WIT implementation bridge for wasmcp Python SDK.

This module provides the bridge between the SDK's high-level API and the
WIT (WebAssembly Interface Types) implementation requirements.
"""

from typing import Any, TYPE_CHECKING

if TYPE_CHECKING:
    from .handler import WasmcpHandler

def register_handler(handler: "WasmcpHandler") -> None:
    """Register a handler with the WIT exports system.
    
    This function is called automatically when a WasmcpHandler is created.
    It ensures the handler's tools, resources, and prompts are available
    to the componentize-py exports system.
    
    Args:
        handler: The WasmcpHandler instance to register
    """
    # Import here to avoid circular imports
    from .exports import register_handler as _register_exports
    _register_exports(handler)

def get_registered_handlers():
    """Get all registered handlers.
    
    Returns:
        Dict containing all registered tools, resources, and prompts
    """
    from .exports import _handler_registry
    return _handler_registry.copy()

def clear_handlers():
    """Clear all registered handlers.
    
    This is mainly useful for testing.
    """
    from .exports import _handler_registry
    _handler_registry["tools"].clear()
    _handler_registry["resources"].clear() 
    _handler_registry["prompts"].clear()