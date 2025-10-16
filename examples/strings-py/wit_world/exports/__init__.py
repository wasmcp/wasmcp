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
        raise NotImplementedError

    @abstractmethod
    def call_tool(self, request: protocol.CallToolRequest, client: protocol.ClientContext) -> Optional[protocol.CallToolResult]:
        raise NotImplementedError


