"""
An interface providing an optional `terminal-input` for stdin as a
link-time authority.
"""
from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some
from ..imports import terminal_input


def get_terminal_stdin() -> Optional[terminal_input.TerminalInput]:
    """
    If stdin is connected to a terminal, return a `terminal-input` handle
    allowing further interaction with it.
    """
    raise NotImplementedError

