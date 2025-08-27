from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some
from ..imports import types

class IncomingHandler(Protocol):

    @abstractmethod
    def handle(self, request: types.IncomingRequest, response_out: types.ResponseOutparam) -> None:
        """
        This function is invoked with an incoming HTTP Request, and a resource
        `response-outparam` which provides the capability to reply with an HTTP
        Response. The response is sent by calling the `response-outparam.set`
        method, which allows execution to continue after the response has been
        sent. This enables both streaming to the response body, and performing other
        work.
        
        The implementor of this function must write a response to the
        `response-outparam` before returning, or else the caller will respond
        with an error on its behalf.
        """
        raise NotImplementedError


