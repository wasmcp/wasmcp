"""
Python MCP Server

An MCP server written in Python
"""

from typing import TYPE_CHECKING

# Import our capability implementations.
# These classes will be instantiated by componentize-py and their methods
# will be called directly by the WebAssembly runtime when handling MCP requests.
from capabilities.lifecycle import Lifecycle
from capabilities.authorization import Authorization  
from capabilities.tools import Tools

# Protocol verification (compile-time only).
# This block ensures our implementations match the WIT-generated Protocol classes.
# The Protocol pattern (PEP 544) provides structural typing - if our classes have
# the right methods with the right signatures, they satisfy the protocol.
# This catches errors at build time via pyright, not at runtime.
if TYPE_CHECKING:
    from wit_world.exports import (
        Lifecycle as LifecycleProtocol,
        Authorization as AuthorizationProtocol,
        Tools as ToolsProtocol
    )
    _lifecycle: LifecycleProtocol = Lifecycle()
    _authorization: AuthorizationProtocol = Authorization()
    _tools: ToolsProtocol = Tools()
