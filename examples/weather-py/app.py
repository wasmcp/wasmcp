"""
Python MCP Weather Server

A WebAssembly component that provides weather tools using the WIT bindings directly.
This implementation follows the same clean architecture as weather-rs.
"""

from typing import TYPE_CHECKING

# Import our implementations
from capabilities.lifecycle import Lifecycle
from capabilities.authorization import Authorization  
from capabilities.tools import Tools

# Verify our implementations satisfy the WIT protocols (for type checking only)
if TYPE_CHECKING:
    from wit_world.exports import (
        Lifecycle as LifecycleProtocol,
        Authorization as AuthorizationProtocol,
        Tools as ToolsProtocol
    )
    _lifecycle: LifecycleProtocol = Lifecycle()
    _authorization: AuthorizationProtocol = Authorization()
    _tools: ToolsProtocol = Tools()
