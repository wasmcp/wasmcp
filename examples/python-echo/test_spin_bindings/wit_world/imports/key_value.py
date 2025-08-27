from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some



@dataclass
class Error_StoreTableFull:
    pass


@dataclass
class Error_NoSuchStore:
    pass


@dataclass
class Error_AccessDenied:
    pass


@dataclass
class Error_Other:
    value: str


Error = Union[Error_StoreTableFull, Error_NoSuchStore, Error_AccessDenied, Error_Other]
"""
The set of errors which may be raised by functions in this interface
"""


class Store:
    """
    An open key-value store
    """
    
    @classmethod
    def open(cls, label: str) -> Self:
        """
        Open the store with the specified label.
        
        `label` must refer to a store allowed in the spin.toml manifest.
        
        `error::no-such-store` will be raised if the `label` is not recognized.
        
        Raises: `wit_world.types.Err(wit_world.imports.key_value.Error)`
        """
        raise NotImplementedError
    def get(self, key: str) -> Optional[bytes]:
        """
        Get the value associated with the specified `key`
        
        Returns `ok(none)` if the key does not exist.
        
        Raises: `wit_world.types.Err(wit_world.imports.key_value.Error)`
        """
        raise NotImplementedError
    def set(self, key: str, value: bytes) -> None:
        """
        Set the `value` associated with the specified `key` overwriting any existing value.
        
        Raises: `wit_world.types.Err(wit_world.imports.key_value.Error)`
        """
        raise NotImplementedError
    def delete(self, key: str) -> None:
        """
        Delete the tuple with the specified `key`
        
        No error is raised if a tuple did not previously exist for `key`.
        
        Raises: `wit_world.types.Err(wit_world.imports.key_value.Error)`
        """
        raise NotImplementedError
    def exists(self, key: str) -> bool:
        """
        Return whether a tuple exists for the specified `key`
        
        Raises: `wit_world.types.Err(wit_world.imports.key_value.Error)`
        """
        raise NotImplementedError
    def get_keys(self) -> List[str]:
        """
        Return a list of all the keys
        
        Raises: `wit_world.types.Err(wit_world.imports.key_value.Error)`
        """
        raise NotImplementedError
    def __enter__(self) -> Self:
        """Returns self"""
        return self
                                
    def __exit__(self, exc_type: type[BaseException] | None, exc_value: BaseException | None, traceback: TracebackType | None) -> bool | None:
        """
        Release this resource.
        """
        raise NotImplementedError



