"""
Stream handling for WASI HTTP responses.
Copied from app.py's Stream class.
"""

import asyncio
from typing import Optional

from .bindings import bindings
from .poll_loop import register

# Try to import Err type
try:
    from wit_world.types import Err
    from wit_world.imports.streams import StreamError_Closed
    from wit_world.imports.wasi_http_types import IncomingBody
except ImportError:
    Err = None
    StreamError_Closed = None
    IncomingBody = None


class Stream:
    """Reader abstraction over wasi:http/types#incoming-body."""
    
    def __init__(self, body):
        self.body = body
        self.stream = body.stream()
    
    async def next(self) -> Optional[bytes]:
        """Wait for the next chunk of data to arrive on the stream."""
        while True:
            try:
                if self.stream is None:
                    return None
                else:
                    buffer = self.stream.read(16 * 1024)
                    if len(buffer) == 0:
                        await register(asyncio.get_event_loop(), self.stream.subscribe())
                    else:
                        return buffer
            except Exception as e:
                # Handle stream closed error
                if Err and isinstance(e, Err):
                    if StreamError_Closed and isinstance(e.value, StreamError_Closed):
                        if self.stream is not None:
                            self.stream.__exit__(None, None, None)
                            self.stream = None
                        if self.body is not None and IncomingBody:
                            IncomingBody.finish(self.body)
                            self.body = None
                        return None
                    else:
                        raise e
                # Fallback for different error types
                elif "closed" in str(e).lower():
                    if self.stream is not None:
                        try:
                            self.stream.__exit__(None, None, None)
                        except:
                            pass
                        self.stream = None
                    if self.body is not None:
                        try:
                            # Try to call finish if available
                            if IncomingBody:
                                IncomingBody.finish(self.body)
                            else:
                                # Try direct method
                                if hasattr(self.body, 'finish'):
                                    self.body.finish()
                        except:
                            pass
                        self.body = None
                    return None
                else:
                    raise
    
    async def read_all(self) -> bytes:
        """Read all data from the stream."""
        chunks = []
        while True:
            chunk = await self.next()
            if chunk is None:
                break
            chunks.append(chunk)
        return b"".join(chunks)