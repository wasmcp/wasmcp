"""
WASI Monotonic Clock is a clock API intended to let users measure elapsed
time.

It is intended to be portable at least between Unix-family platforms and
Windows.

A monotonic clock is a clock which has an unspecified initial value, and
successive reads of the clock will produce non-decreasing values.
"""
from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some
from ..imports import poll


def now() -> int:
    """
    Read the current value of the clock.
    
    The clock is monotonic, therefore calling this function repeatedly will
    produce a sequence of non-decreasing values.
    """
    raise NotImplementedError

def resolution() -> int:
    """
    Query the resolution of the clock. Returns the duration of time
    corresponding to a clock tick.
    """
    raise NotImplementedError

def subscribe_instant(when: int) -> poll.Pollable:
    """
    Create a `pollable` which will resolve once the specified instant
    has occurred.
    """
    raise NotImplementedError

def subscribe_duration(when: int) -> poll.Pollable:
    """
    Create a `pollable` that will resolve after the specified duration has
    elapsed from the time this function is invoked.
    """
    raise NotImplementedError

