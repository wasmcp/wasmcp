from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some



@dataclass
class Error_InvalidAddress:
    pass


@dataclass
class Error_TooManyConnections:
    pass


@dataclass
class Error_TypeError:
    pass


@dataclass
class Error_Other:
    value: str


Error = Union[Error_InvalidAddress, Error_TooManyConnections, Error_TypeError, Error_Other]
"""
Errors related to interacting with Redis
"""



@dataclass
class RedisParameter_Int64:
    value: int


@dataclass
class RedisParameter_Binary:
    value: bytes


RedisParameter = Union[RedisParameter_Int64, RedisParameter_Binary]
"""
A parameter type for the general-purpose `execute` function.
"""



@dataclass
class RedisResult_Nil:
    pass


@dataclass
class RedisResult_Status:
    value: str


@dataclass
class RedisResult_Int64:
    value: int


@dataclass
class RedisResult_Binary:
    value: bytes


RedisResult = Union[RedisResult_Nil, RedisResult_Status, RedisResult_Int64, RedisResult_Binary]
"""
A return type for the general-purpose `execute` function.
"""


class Connection:
    
    @classmethod
    def open(cls, address: str) -> Self:
        """
        Open a connection to the Redis instance at `address`.
        
        Raises: `wit_world.types.Err(wit_world.imports.redis.Error)`
        """
        raise NotImplementedError
    def publish(self, channel: str, payload: bytes) -> None:
        """
        Publish a Redis message to the specified channel.
        
        Raises: `wit_world.types.Err(wit_world.imports.redis.Error)`
        """
        raise NotImplementedError
    def get(self, key: str) -> Optional[bytes]:
        """
        Get the value of a key.
        
        Raises: `wit_world.types.Err(wit_world.imports.redis.Error)`
        """
        raise NotImplementedError
    def set(self, key: str, value: bytes) -> None:
        """
        Set key to value.
        
        If key already holds a value, it is overwritten.
        
        Raises: `wit_world.types.Err(wit_world.imports.redis.Error)`
        """
        raise NotImplementedError
    def incr(self, key: str) -> int:
        """
        Increments the number stored at key by one.
        
        If the key does not exist, it is set to 0 before performing the operation.
        An `error::type-error` is returned if the key contains a value of the wrong type
        or contains a string that can not be represented as integer.
        
        Raises: `wit_world.types.Err(wit_world.imports.redis.Error)`
        """
        raise NotImplementedError
    def del_(self, keys: List[str]) -> int:
        """
        Removes the specified keys.
        
        A key is ignored if it does not exist. Returns the number of keys deleted.
        
        Raises: `wit_world.types.Err(wit_world.imports.redis.Error)`
        """
        raise NotImplementedError
    def sadd(self, key: str, values: List[str]) -> int:
        """
        Add the specified `values` to the set named `key`, returning the number of newly-added values.
        
        Raises: `wit_world.types.Err(wit_world.imports.redis.Error)`
        """
        raise NotImplementedError
    def smembers(self, key: str) -> List[str]:
        """
        Retrieve the contents of the set named `key`.
        
        Raises: `wit_world.types.Err(wit_world.imports.redis.Error)`
        """
        raise NotImplementedError
    def srem(self, key: str, values: List[str]) -> int:
        """
        Remove the specified `values` from the set named `key`, returning the number of newly-removed values.
        
        Raises: `wit_world.types.Err(wit_world.imports.redis.Error)`
        """
        raise NotImplementedError
    def execute(self, command: str, arguments: List[RedisParameter]) -> List[RedisResult]:
        """
        Execute an arbitrary Redis command and receive the result.
        
        Raises: `wit_world.types.Err(wit_world.imports.redis.Error)`
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



