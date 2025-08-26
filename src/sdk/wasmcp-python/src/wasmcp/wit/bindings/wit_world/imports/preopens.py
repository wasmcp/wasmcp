from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some
from ..imports import wasi_filesystem_types


def get_directories() -> List[Tuple[wasi_filesystem_types.Descriptor, str]]:
    """
    Return the set of preopened directories, and their path.
    """
    raise NotImplementedError

