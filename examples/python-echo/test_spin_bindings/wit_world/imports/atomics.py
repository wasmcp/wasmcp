"""
A keyvalue interface that provides atomic operations.

Atomic operations are single, indivisible operations. When a fault causes an atomic operation to
fail, it will appear to the invoker of the atomic operation that the action either completed
successfully or did nothing at all.

Please note that this interface is bare functions that take a reference to a bucket. This is to
get around the current lack of a way to "extend" a resource with additional methods inside of
wit. Future version of the interface will instead extend these methods on the base `bucket`
resource.
"""
from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some
from ..imports import wasi_keyvalue_store

class Cas:
    """
    A handle to a CAS (compare-and-swap) operation.
    """
    
    @classmethod
    def new(cls, bucket: wasi_keyvalue_store.Bucket, key: str) -> Self:
        """
        Construct a new CAS operation. Implementors can map the underlying functionality
        (transactions, versions, etc) as desired.
        
        Raises: `wit_world.types.Err(wit_world.imports.wasi_keyvalue_store.Error)`
        """
        raise NotImplementedError
    def current(self) -> Optional[bytes]:
        """
        Get the current value of the key (if it exists). This allows for avoiding reads if all
        that is needed to ensure the atomicity of the operation
        
        Raises: `wit_world.types.Err(wit_world.imports.wasi_keyvalue_store.Error)`
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



@dataclass
class CasError_StoreError:
    value: wasi_keyvalue_store.Error


@dataclass
class CasError_CasFailed:
    value: Cas


CasError = Union[CasError_StoreError, CasError_CasFailed]
"""
The error returned by a CAS operation
"""



def increment(bucket: wasi_keyvalue_store.Bucket, key: str, delta: int) -> int:
    """
    Atomically increment the value associated with the key in the store by the given delta. It
    returns the new value.
    
    If the key does not exist in the store, it creates a new key-value pair with the value set
    to the given delta.
    
    If any other error occurs, it returns an `Err(error)`.
    
    Raises: `wit_world.types.Err(wit_world.imports.wasi_keyvalue_store.Error)`
    """
    raise NotImplementedError

def swap(cas: Cas, value: bytes) -> None:
    """
    Perform the swap on a CAS operation. This consumes the CAS handle and returns an error if
    the CAS operation failed.
    
    Raises: `wit_world.types.Err(wit_world.imports.atomics.CasError)`
    """
    raise NotImplementedError

