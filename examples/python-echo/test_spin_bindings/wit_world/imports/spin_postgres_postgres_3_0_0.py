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
    FLOATING32 = 5
    FLOATING64 = 6
    STR = 7
    BINARY = 8
    DATE = 9
    TIME = 10
    DATETIME = 11
    TIMESTAMP = 12
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
class DbValue_Date:
    value: Tuple[int, int, int]


@dataclass
class DbValue_Time:
    value: Tuple[int, int, int, int]


@dataclass
class DbValue_Datetime:
    value: Tuple[int, int, int, int, int, int, int]


@dataclass
class DbValue_Timestamp:
    value: int


@dataclass
class DbValue_DbNull:
    pass


@dataclass
class DbValue_Unsupported:
    pass


DbValue = Union[DbValue_Boolean, DbValue_Int8, DbValue_Int16, DbValue_Int32, DbValue_Int64, DbValue_Floating32, DbValue_Floating64, DbValue_Str, DbValue_Binary, DbValue_Date, DbValue_Time, DbValue_Datetime, DbValue_Timestamp, DbValue_DbNull, DbValue_Unsupported]
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
class ParameterValue_Date:
    value: Tuple[int, int, int]


@dataclass
class ParameterValue_Time:
    value: Tuple[int, int, int, int]


@dataclass
class ParameterValue_Datetime:
    value: Tuple[int, int, int, int, int, int, int]


@dataclass
class ParameterValue_Timestamp:
    value: int


@dataclass
class ParameterValue_DbNull:
    pass


ParameterValue = Union[ParameterValue_Boolean, ParameterValue_Int8, ParameterValue_Int16, ParameterValue_Int32, ParameterValue_Int64, ParameterValue_Floating32, ParameterValue_Floating64, ParameterValue_Str, ParameterValue_Binary, ParameterValue_Date, ParameterValue_Time, ParameterValue_Datetime, ParameterValue_Timestamp, ParameterValue_DbNull]
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

class Connection:
    """
    A connection to a postgres database.
    """
    
    @classmethod
    def open(cls, address: str) -> Self:
        """
        Open a connection to the Postgres instance at `address`.
        
        Raises: `wit_world.types.Err(wit_world.imports.spin_postgres_postgres_3_0_0.Error)`
        """
        raise NotImplementedError
    def query(self, statement: str, params: List[ParameterValue]) -> RowSet:
        """
        Query the database.
        
        Raises: `wit_world.types.Err(wit_world.imports.spin_postgres_postgres_3_0_0.Error)`
        """
        raise NotImplementedError
    def execute(self, statement: str, params: List[ParameterValue]) -> int:
        """
        Execute command to the database.
        
        Raises: `wit_world.types.Err(wit_world.imports.spin_postgres_postgres_3_0_0.Error)`
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



