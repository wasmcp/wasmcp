"""
This interface defines a handler of outgoing HTTP Requests. It should be
imported by components which wish to make HTTP Requests.
"""
from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some
from ..imports import types


def handle(request: types.OutgoingRequest, options: Optional[types.RequestOptions]) -> types.FutureIncomingResponse:
    """
    This function is invoked with an outgoing HTTP Request, and it returns
    a resource `future-incoming-response` which represents an HTTP Response
    which may arrive in the future.
    
    The `options` argument accepts optional parameters for the HTTP
    protocol's transport layer.
    
    This function may return an error if the `outgoing-request` is invalid
    or not allowed to be made. Otherwise, protocol errors are reported
    through the `future-incoming-response`.
    
    Raises: `wit_world.types.Err(wit_world.imports.types.ErrorCode)`
    """
    raise NotImplementedError

