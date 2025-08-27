from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some



@dataclass
class Error_InvalidName:
    value: str


@dataclass
class Error_Undefined:
    value: str


@dataclass
class Error_Provider:
    value: str


@dataclass
class Error_Other:
    value: str


Error = Union[Error_InvalidName, Error_Undefined, Error_Provider, Error_Other]
"""
The set of errors which may be raised by functions in this interface.
"""



def get(name: str) -> str:
    """
    Get an application variable value for the current component.
    
    The name must match one defined in in the component manifest.
    
    Raises: `wit_world.types.Err(wit_world.imports.variables.Error)`
    """
    raise NotImplementedError

