from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some


@dataclass
class DbError:
    as_text: str
    severity: str
    code: str
    message: str
    detail: Optional[str]
    extras: List[Tuple[str, str]]


@dataclass
class QueryError_Text:
    value: str


@dataclass
class QueryError_DbError:
    value: DbError


QueryError = Union[QueryError_Text, QueryError_DbError]



@dataclass
class Error_ConnectionFailed:
    value: str


@dataclass
class Error_BadParameter:
    value: str


@dataclass
class Error_QueryFailed:
    value: QueryError


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



@dataclass
class DbDataType_Boolean:
    pass


@dataclass
class DbDataType_Int8:
    pass


@dataclass
class DbDataType_Int16:
    pass


@dataclass
class DbDataType_Int32:
    pass


@dataclass
class DbDataType_Int64:
    pass


@dataclass
class DbDataType_Floating32:
    pass


@dataclass
class DbDataType_Floating64:
    pass


@dataclass
class DbDataType_Str:
    pass


@dataclass
class DbDataType_Binary:
    pass


@dataclass
class DbDataType_Date:
    pass


@dataclass
class DbDataType_Time:
    pass


@dataclass
class DbDataType_Datetime:
    pass


@dataclass
class DbDataType_Timestamp:
    pass


@dataclass
class DbDataType_Uuid:
    pass


@dataclass
class DbDataType_Jsonb:
    pass


@dataclass
class DbDataType_Decimal:
    pass


@dataclass
class DbDataType_RangeInt32:
    pass


@dataclass
class DbDataType_RangeInt64:
    pass


@dataclass
class DbDataType_RangeDecimal:
    pass


@dataclass
class DbDataType_ArrayInt32:
    pass


@dataclass
class DbDataType_ArrayInt64:
    pass


@dataclass
class DbDataType_ArrayDecimal:
    pass


@dataclass
class DbDataType_ArrayStr:
    pass


@dataclass
class DbDataType_Interval:
    pass


@dataclass
class DbDataType_Other:
    value: str


DbDataType = Union[DbDataType_Boolean, DbDataType_Int8, DbDataType_Int16, DbDataType_Int32, DbDataType_Int64, DbDataType_Floating32, DbDataType_Floating64, DbDataType_Str, DbDataType_Binary, DbDataType_Date, DbDataType_Time, DbDataType_Datetime, DbDataType_Timestamp, DbDataType_Uuid, DbDataType_Jsonb, DbDataType_Decimal, DbDataType_RangeInt32, DbDataType_RangeInt64, DbDataType_RangeDecimal, DbDataType_ArrayInt32, DbDataType_ArrayInt64, DbDataType_ArrayDecimal, DbDataType_ArrayStr, DbDataType_Interval, DbDataType_Other]
"""
Data types for a database column
"""


@dataclass
class Interval:
    micros: int
    days: int
    months: int

@dataclass
class Column:
    """
    A database column
    """
    name: str
    data_type: DbDataType

class RangeBoundKind(Enum):
    """
    For range types, indicates if each bound is inclusive or exclusive
    """
    INCLUSIVE = 0
    EXCLUSIVE = 1


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
class DbValue_Uuid:
    value: str


@dataclass
class DbValue_Jsonb:
    value: bytes


@dataclass
class DbValue_Decimal:
    value: str


@dataclass
class DbValue_RangeInt32:
    value: Tuple[Optional[Tuple[int, RangeBoundKind]], Optional[Tuple[int, RangeBoundKind]]]


@dataclass
class DbValue_RangeInt64:
    value: Tuple[Optional[Tuple[int, RangeBoundKind]], Optional[Tuple[int, RangeBoundKind]]]


@dataclass
class DbValue_RangeDecimal:
    value: Tuple[Optional[Tuple[str, RangeBoundKind]], Optional[Tuple[str, RangeBoundKind]]]


@dataclass
class DbValue_ArrayInt32:
    value: List[Optional[int]]


@dataclass
class DbValue_ArrayInt64:
    value: List[Optional[int]]


@dataclass
class DbValue_ArrayDecimal:
    value: List[Optional[str]]


@dataclass
class DbValue_ArrayStr:
    value: List[Optional[str]]


@dataclass
class DbValue_Interval:
    value: Interval


@dataclass
class DbValue_DbNull:
    pass


@dataclass
class DbValue_Unsupported:
    value: bytes


DbValue = Union[DbValue_Boolean, DbValue_Int8, DbValue_Int16, DbValue_Int32, DbValue_Int64, DbValue_Floating32, DbValue_Floating64, DbValue_Str, DbValue_Binary, DbValue_Date, DbValue_Time, DbValue_Datetime, DbValue_Timestamp, DbValue_Uuid, DbValue_Jsonb, DbValue_Decimal, DbValue_RangeInt32, DbValue_RangeInt64, DbValue_RangeDecimal, DbValue_ArrayInt32, DbValue_ArrayInt64, DbValue_ArrayDecimal, DbValue_ArrayStr, DbValue_Interval, DbValue_DbNull, DbValue_Unsupported]
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
class ParameterValue_Uuid:
    value: str


@dataclass
class ParameterValue_Jsonb:
    value: bytes


@dataclass
class ParameterValue_Decimal:
    value: str


@dataclass
class ParameterValue_RangeInt32:
    value: Tuple[Optional[Tuple[int, RangeBoundKind]], Optional[Tuple[int, RangeBoundKind]]]


@dataclass
class ParameterValue_RangeInt64:
    value: Tuple[Optional[Tuple[int, RangeBoundKind]], Optional[Tuple[int, RangeBoundKind]]]


@dataclass
class ParameterValue_RangeDecimal:
    value: Tuple[Optional[Tuple[str, RangeBoundKind]], Optional[Tuple[str, RangeBoundKind]]]


@dataclass
class ParameterValue_ArrayInt32:
    value: List[Optional[int]]


@dataclass
class ParameterValue_ArrayInt64:
    value: List[Optional[int]]


@dataclass
class ParameterValue_ArrayDecimal:
    value: List[Optional[str]]


@dataclass
class ParameterValue_ArrayStr:
    value: List[Optional[str]]


@dataclass
class ParameterValue_Interval:
    value: Interval


@dataclass
class ParameterValue_DbNull:
    pass


ParameterValue = Union[ParameterValue_Boolean, ParameterValue_Int8, ParameterValue_Int16, ParameterValue_Int32, ParameterValue_Int64, ParameterValue_Floating32, ParameterValue_Floating64, ParameterValue_Str, ParameterValue_Binary, ParameterValue_Date, ParameterValue_Time, ParameterValue_Datetime, ParameterValue_Timestamp, ParameterValue_Uuid, ParameterValue_Jsonb, ParameterValue_Decimal, ParameterValue_RangeInt32, ParameterValue_RangeInt64, ParameterValue_RangeDecimal, ParameterValue_ArrayInt32, ParameterValue_ArrayInt64, ParameterValue_ArrayDecimal, ParameterValue_ArrayStr, ParameterValue_Interval, ParameterValue_DbNull]
"""
Values used in parameterized queries
"""


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
        
        Raises: `wit_world.types.Err(wit_world.imports.spin_postgres_postgres_4_0_0.Error)`
        """
        raise NotImplementedError
    def query(self, statement: str, params: List[ParameterValue]) -> RowSet:
        """
        Query the database.
        
        Raises: `wit_world.types.Err(wit_world.imports.spin_postgres_postgres_4_0_0.Error)`
        """
        raise NotImplementedError
    def execute(self, statement: str, params: List[ParameterValue]) -> int:
        """
        Execute command to the database.
        
        Raises: `wit_world.types.Err(wit_world.imports.spin_postgres_postgres_4_0_0.Error)`
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



