from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some



@dataclass
class Error_Upstream:
    value: str


@dataclass
class Error_Io:
    value: str


Error = Union[Error_Upstream, Error_Io]
"""
An error type that encapsulates the different errors that can occur fetching configuration values.
"""



def get(key: str) -> Optional[str]:
    """
    Gets a configuration value of type `string` associated with the `key`.
    
    The value is returned as an `option<string>`. If the key is not found,
    `Ok(none)` is returned. If an error occurs, an `Err(error)` is returned.
    
    Raises: `wit_world.types.Err(wit_world.imports.wasi_config_store.Error)`
    """
    raise NotImplementedError

def get_all() -> List[Tuple[str, str]]:
    """
    Gets a list of configuration key-value pairs of type `string`.
    
    If an error occurs, an `Err(error)` is returned.
    
    Raises: `wit_world.types.Err(wit_world.imports.wasi_config_store.Error)`
    """
    raise NotImplementedError

