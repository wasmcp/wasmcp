from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some
from ..imports import protocol

class ToolsCapability(Protocol):

    @abstractmethod
    def list_tools(self, request: protocol.ListToolsRequest, client: protocol.ClientContext) -> protocol.ListToolsResult:
        """
        List all tools provided by this capability
        
        This function returns the complete catalog of tools this component can execute.
        The middleware will automatically merge these tools with tools from other
        capability components in the pipeline.
        
        The request may include pagination parameters (cursor) which should be honored
        if the component has a large number of tools.
        
        <https://spec.modelcontextprotocol.io/specification/server/tools#tools-list>
        """
        raise NotImplementedError

    @abstractmethod
    def call_tool(self, request: protocol.CallToolRequest, client: protocol.ClientContext) -> Optional[protocol.CallToolResult]:
        """
        Execute a tool call
        
        This function is called when a client invokes a tool. The component should:
        1. Check if the tool name matches one of its tools
        2. If yes, execute the tool and return Some(result)
        3. If no, return None to indicate this capability doesn't handle this tool
        
        Returning None allows the middleware to delegate the call to the next
        capability in the pipeline. This enables automatic composition of multiple
        tool providers without manual routing logic.
        
        The middleware handles all error cases:
        - Tool not found (when all capabilities return None)
        - Invalid arguments (component returns error in result)
        - Execution failures (component returns error in result)
        
        <https://spec.modelcontextprotocol.io/specification/server/tools#tools-call>
        """
        raise NotImplementedError


