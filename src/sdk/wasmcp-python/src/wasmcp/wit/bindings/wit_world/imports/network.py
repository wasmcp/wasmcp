from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some


class Network:
    """
    An opaque resource that represents access to (a subset of) the network.
    This enables context-based security for networking.
    There is no need for this to map 1:1 to a physical network interface.
    """
    
    def __enter__(self) -> Self:
        """Returns self"""
        return self
                                
    def __exit__(self, exc_type: type[BaseException] | None, exc_value: BaseException | None, traceback: TracebackType | None) -> bool | None:
        """
        Release this resource.
        """
        raise NotImplementedError


class ErrorCode(Enum):
    """
    Error codes.
    
    In theory, every API can return any error code.
    In practice, API's typically only return the errors documented per API
    combined with a couple of errors that are always possible:
    - `unknown`
    - `access-denied`
    - `not-supported`
    - `out-of-memory`
    - `concurrency-conflict`
    
    See each individual API for what the POSIX equivalents are. They sometimes differ per API.
    """
    UNKNOWN = 0
    ACCESS_DENIED = 1
    NOT_SUPPORTED = 2
    INVALID_ARGUMENT = 3
    OUT_OF_MEMORY = 4
    TIMEOUT = 5
    CONCURRENCY_CONFLICT = 6
    NOT_IN_PROGRESS = 7
    WOULD_BLOCK = 8
    INVALID_STATE = 9
    NEW_SOCKET_LIMIT = 10
    ADDRESS_NOT_BINDABLE = 11
    ADDRESS_IN_USE = 12
    REMOTE_UNREACHABLE = 13
    CONNECTION_REFUSED = 14
    CONNECTION_RESET = 15
    CONNECTION_ABORTED = 16
    DATAGRAM_TOO_LARGE = 17
    NAME_UNRESOLVABLE = 18
    TEMPORARY_RESOLVER_FAILURE = 19
    PERMANENT_RESOLVER_FAILURE = 20

class IpAddressFamily(Enum):
    IPV4 = 0
    IPV6 = 1


@dataclass
class IpAddress_Ipv4:
    value: Tuple[int, int, int, int]


@dataclass
class IpAddress_Ipv6:
    value: Tuple[int, int, int, int, int, int, int, int]


IpAddress = Union[IpAddress_Ipv4, IpAddress_Ipv6]


@dataclass
class Ipv4SocketAddress:
    port: int
    address: Tuple[int, int, int, int]

@dataclass
class Ipv6SocketAddress:
    port: int
    flow_info: int
    address: Tuple[int, int, int, int, int, int, int, int]
    scope_id: int


@dataclass
class IpSocketAddress_Ipv4:
    value: Ipv4SocketAddress


@dataclass
class IpSocketAddress_Ipv6:
    value: Ipv6SocketAddress


IpSocketAddress = Union[IpSocketAddress_Ipv4, IpSocketAddress_Ipv6]



