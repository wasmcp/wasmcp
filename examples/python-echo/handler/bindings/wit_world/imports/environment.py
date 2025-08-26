from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some



def get_environment() -> List[Tuple[str, str]]:
    """
    Get the POSIX-style environment variables.
    
    Each environment variable is provided as a pair of string variable names
    and string value.
    
    Morally, these are a value import, but until value imports are available
    in the component model, this import function should return the same
    values each time it is called.
    """
    raise NotImplementedError

def get_arguments() -> List[str]:
    """
    Get the POSIX-style arguments to the program.
    """
    raise NotImplementedError

def initial_cwd() -> Optional[str]:
    """
    Return a path that programs should use as their initial current working
    directory, interpreting `.` as shorthand for this.
    """
    raise NotImplementedError

