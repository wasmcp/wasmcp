from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some


@dataclass
class Tool:
    name: str
    description: str
    input_schema: str

@dataclass
class ResourceInfo:
    uri: str
    name: str
    description: Optional[str]
    mime_type: Optional[str]

@dataclass
class ResourceContents:
    uri: str
    mime_type: Optional[str]
    text: Optional[str]
    blob: Optional[bytes]

@dataclass
class PromptArgument:
    name: str
    description: Optional[str]
    required: bool

@dataclass
class Prompt:
    name: str
    description: Optional[str]
    arguments: List[PromptArgument]

@dataclass
class PromptMessage:
    role: str
    content: str

@dataclass
class Error:
    code: int
    message: str
    data: Optional[str]


@dataclass
class ToolResult_Text:
    value: str


@dataclass
class ToolResult_Error:
    value: Error


ToolResult = Union[ToolResult_Text, ToolResult_Error]



@dataclass
class ResourceResult_Contents:
    value: ResourceContents


@dataclass
class ResourceResult_Error:
    value: Error


ResourceResult = Union[ResourceResult_Contents, ResourceResult_Error]



@dataclass
class PromptResult_Messages:
    value: List[PromptMessage]


@dataclass
class PromptResult_Error:
    value: Error


PromptResult = Union[PromptResult_Messages, PromptResult_Error]


