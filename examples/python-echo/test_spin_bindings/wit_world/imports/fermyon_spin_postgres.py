from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some
from ..imports import rdbms_types

class Connection:
    """
    A connection to a postgres database.
    """
    
    @classmethod
    def open(cls, address: str) -> Self:
        """
        Open a connection to the Postgres instance at `address`.
        
        Raises: `wit_world.types.Err(wit_world.imports.rdbms_types.Error)`
        """
        raise NotImplementedError
    def query(self, statement: str, params: List[rdbms_types.ParameterValue]) -> rdbms_types.RowSet:
        """
        Query the database.
        
        Raises: `wit_world.types.Err(wit_world.imports.rdbms_types.Error)`
        """
        raise NotImplementedError
    def execute(self, statement: str, params: List[rdbms_types.ParameterValue]) -> int:
        """
        Execute command to the database.
        
        Raises: `wit_world.types.Err(wit_world.imports.rdbms_types.Error)`
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



