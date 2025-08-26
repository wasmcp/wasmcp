from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some
from ..exports import handler

class Handler(Protocol):

    @abstractmethod
    def list_tools(self) -> List[handler.Tool]:
        raise NotImplementedError

    @abstractmethod
    def call_tool(self, name: str, arguments: str) -> handler.ToolResult:
        raise NotImplementedError

    @abstractmethod
    def list_resources(self) -> List[handler.ResourceInfo]:
        raise NotImplementedError

    @abstractmethod
    def read_resource(self, uri: str) -> handler.ResourceResult:
        raise NotImplementedError

    @abstractmethod
    def list_prompts(self) -> List[handler.Prompt]:
        raise NotImplementedError

    @abstractmethod
    def get_prompt(self, name: str, arguments: str) -> handler.PromptResult:
        raise NotImplementedError


