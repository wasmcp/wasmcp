"""Type stubs for componentize-py's poll_loop module."""

from typing import Any, Optional, TypeVar, Coroutine
from asyncio import AbstractEventLoop

T = TypeVar('T')

class PollLoop(AbstractEventLoop):
    """Custom asyncio event loop backed by WASI polling."""
    def run_until_complete(self, future: Coroutine[Any, Any, T]) -> T: ...
    def close(self) -> None: ...

class Stream:
    """Reader abstraction over wasi:http/types#incoming-body."""
    def __init__(self, body: Any) -> None: ...
    async def next(self) -> Optional[bytes]: ...

async def send(request: Any) -> Any: ...