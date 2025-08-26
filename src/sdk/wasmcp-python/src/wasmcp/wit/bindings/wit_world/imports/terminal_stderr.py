"""
An interface providing an optional `terminal-output` for stderr as a
link-time authority.
"""
from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some
from ..imports import terminal_output


def get_terminal_stderr() -> Optional[terminal_output.TerminalOutput]:
    """
    If stderr is connected to a terminal, return a `terminal-output` handle
    allowing further interaction with it.
    """
    raise NotImplementedError

