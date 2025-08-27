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
class Error_ConnectionFailed:
    value: str


@dataclass
class Error_Other:
    value: str


Error = Union[Error_InvalidAddress, Error_TooManyConnections, Error_ConnectionFailed, Error_Other]
"""
Errors related to interacting with Mqtt
"""


class Qos(Enum):
    """
    QoS for publishing Mqtt messages
    """
    AT_MOST_ONCE = 0
    AT_LEAST_ONCE = 1
    EXACTLY_ONCE = 2

class Connection:
    
    @classmethod
    def open(cls, address: str, username: str, password: str, keep_alive_interval_in_secs: int) -> Self:
        """
        Open a connection to the Mqtt instance at `address`.
        
        Raises: `wit_world.types.Err(wit_world.imports.mqtt.Error)`
        """
        raise NotImplementedError
    def publish(self, topic: str, payload: bytes, qos: Qos) -> None:
        """
        Publish an Mqtt message to the specified `topic`.
        
        Raises: `wit_world.types.Err(wit_world.imports.mqtt.Error)`
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



