from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some



@dataclass
class Error_ConnectionFailed:
    value: str


@dataclass
class Error_BadParameter:
    value: str


@dataclass
class Error_QueryFailed:
    value: str


@dataclass
class Error_ValueConversionFailed:
    value: str


@dataclass
class Error_Other:
    value: str


Error = Union[Error_ConnectionFailed, Error_BadParameter, Error_QueryFailed, Error_ValueConversionFailed, Error_Other]
"""
Errors related to interacting with a database.
"""


class DbDataType(Enum):
    """
    Data types for a database column
    """
    BOOLEAN = 0
    INT8 = 1
    INT16 = 2
    INT32 = 3
    INT64 = 4
    UINT8 = 5
    UINT16 = 6
    UINT32 = 7
    UINT64 = 8
    FLOATING32 = 9
    FLOATING64 = 10
    STR = 11
    BINARY = 12
    OTHER = 13


@dataclass
class DbValue_Boolean:
    value: bool


@dataclass
class DbValue_Int8:
    value: int


@dataclass
class DbValue_Int16:
    value: int


@dataclass
class DbValue_Int32:
    value: int


@dataclass
class DbValue_Int64:
    value: int


@dataclass
class DbValue_Uint8:
    value: int


@dataclass
class DbValue_Uint16:
    value: int


@dataclass
class DbValue_Uint32:
    value: int


@dataclass
class DbValue_Uint64:
    value: int


@dataclass
class DbValue_Floating32:
    value: float


@dataclass
class DbValue_Floating64:
    value: float


@dataclass
class DbValue_Str:
    value: str


@dataclass
class DbValue_Binary:
    value: bytes


@dataclass
class DbValue_DbNull:
    pass


@dataclass
class DbValue_Unsupported:
    pass


DbValue = Union[DbValue_Boolean, DbValue_Int8, DbValue_Int16, DbValue_Int32, DbValue_Int64, DbValue_Uint8, DbValue_Uint16, DbValue_Uint32, DbValue_Uint64, DbValue_Floating32, DbValue_Floating64, DbValue_Str, DbValue_Binary, DbValue_DbNull, DbValue_Unsupported]
"""
Database values
"""



@dataclass
class ParameterValue_Boolean:
    value: bool


@dataclass
class ParameterValue_Int8:
    value: int


@dataclass
class ParameterValue_Int16:
    value: int


@dataclass
class ParameterValue_Int32:
    value: int


@dataclass
class ParameterValue_Int64:
    value: int


@dataclass
class ParameterValue_Uint8:
    value: int


@dataclass
class ParameterValue_Uint16:
    value: int


@dataclass
class ParameterValue_Uint32:
    value: int


@dataclass
class ParameterValue_Uint64:
    value: int


@dataclass
class ParameterValue_Floating32:
    value: float


@dataclass
class ParameterValue_Floating64:
    value: float


@dataclass
class ParameterValue_Str:
    value: str


@dataclass
class ParameterValue_Binary:
    value: bytes


@dataclass
class ParameterValue_DbNull:
    pass


ParameterValue = Union[ParameterValue_Boolean, ParameterValue_Int8, ParameterValue_Int16, ParameterValue_Int32, ParameterValue_Int64, ParameterValue_Uint8, ParameterValue_Uint16, ParameterValue_Uint32, ParameterValue_Uint64, ParameterValue_Floating32, ParameterValue_Floating64, ParameterValue_Str, ParameterValue_Binary, ParameterValue_DbNull]
"""
Values used in parameterized queries
"""


@dataclass
class Column:
    """
    A database column
    """
    name: str
    data_type: DbDataType

@dataclass
class RowSet:
    """
    A set of database rows
    """
    columns: List[Column]
    rows: List[List[DbValue]]


