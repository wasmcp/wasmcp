"""
Stream handling for WASI HTTP responses.
"""

import asyncio
from typing import Optional, Generator

from .bindings import bindings
from .poll_loop import register


class Stream:
    """Async-friendly wrapper around WASI input stream."""
    
    def __init__(self, stream):
        self._stream = stream
        
    async def read(self, n: int) -> bytes:
        """Read up to n bytes from the stream."""
        loop = asyncio.get_event_loop()
        
        while True:
            try:
                data = self._stream.read(n)
                if data:
                    return bytes(data)
                
                # Stream might have more data, wait for it
                pollable = self._stream.subscribe()
                await register(loop, pollable)
                
            except Exception as e:
                # Check if it's end of stream
                if "closed" in str(e).lower():
                    return b""
                raise
    
    async def read_all(self) -> bytes:
        """Read all data from the stream."""
        chunks = []
        while True:
            chunk = await self.read(65536)  # 64KB chunks
            if not chunk:
                break
            chunks.append(chunk)
        return b"".join(chunks)
    
    def __enter__(self):
        return self
    
    def __exit__(self, *args):
        # Stream cleanup handled by WASI runtime
        pass