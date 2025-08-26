"""
This interface provides a value-export of the default network handle..
"""
from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some
from ..imports import network


def instance_network() -> network.Network:
    """
    Get a handle to the default network.
    """
    raise NotImplementedError

