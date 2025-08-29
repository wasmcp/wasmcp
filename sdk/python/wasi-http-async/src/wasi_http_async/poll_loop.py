"""
Custom asyncio event loop backed by wasi:io/poll.
Based on Spin SDK implementation.
"""

import asyncio
from asyncio import futures
from collections.abc import Generator
from typing import Any, Optional

from .bindings import bindings


class PollLoop(asyncio.AbstractEventLoop):
    """Custom asyncio event loop implementation backed by wasi:io/poll."""

    def __init__(self):
        self._timer = 0
        self._scheduled = []
        self._ready = []
        self._futures = {}
        self._running = False
        self._stopping = False
        self._closed = False
        asyncio.set_event_loop(self)

    def time(self):
        return self._timer

    def _add_callback(self, handle):
        self._ready.append(handle)

    def call_soon(self, callback, *args, context=None):
        handle = asyncio.Handle(callback, args, self, context)
        self._add_callback(handle)
        return handle

    def call_later(self, delay, callback, *args, context=None):
        if delay <= 0:
            return self.call_soon(callback, *args, context=context)
        
        when = self._timer + delay
        timer_handle = asyncio.TimerHandle(when, callback, args, self, context)
        self._scheduled.append(timer_handle)
        self._scheduled.sort(key=lambda h: h.when())
        return timer_handle

    def call_at(self, when, callback, *args, context=None):
        delay = when - self._timer
        return self.call_later(delay, callback, *args, context=context)

    def create_future(self):
        return futures.Future(loop=self)

    def create_task(self, coro, *, name=None, context=None):
        if not asyncio.iscoroutine(coro):
            raise TypeError('a coroutine was expected')
        
        task = asyncio.Task(coro, loop=self, name=name, context=context)
        return task

    def stop(self):
        self._stopping = True

    def is_running(self):
        return self._running

    def is_closed(self):
        return self._closed
    
    def get_debug(self):
        """Return debug mode status."""
        return False

    def close(self):
        if self._running:
            raise RuntimeError("Cannot close a running event loop")
        if self._closed:
            return
        self._closed = True
        self._ready.clear()
        self._scheduled.clear()

    def shutdown_asyncgens(self):
        pass

    def shutdown_default_executor(self):
        pass

    def _run_once(self):
        # Process scheduled callbacks
        while self._scheduled and self._scheduled[0].when() <= self._timer:
            handle = self._scheduled.pop(0)
            self._ready.append(handle)

        # Process ready callbacks
        ready_count = len(self._ready)
        for _ in range(ready_count):
            if not self._ready:
                break
            handle = self._ready.pop(0)
            if not handle.cancelled():
                handle._run()

        # Handle polling if we have futures
        if self._futures:
            pollables = []
            pollable_futures = []
            
            for future, pollable in self._futures.items():
                if not future.done():
                    pollables.append(pollable)
                    pollable_futures.append(future)

            if pollables:
                ready_list = bindings.poll.poll(pollables)
                for index in ready_list:
                    future = pollable_futures[index]
                    if not future.done():
                        future.set_result(None)

        self._timer += 0.001

    def run_until_complete(self, future):
        if self._running:
            raise RuntimeError("Event loop is already running")
        
        if not asyncio.isfuture(future):
            future = asyncio.ensure_future(future, loop=self)

        future.add_done_callback(lambda f: self.stop())
        
        self._running = True
        self._stopping = False
        
        try:
            while not self._stopping:
                self._run_once()
                if future.done():
                    break
            
            if not future.done():
                raise RuntimeError("Event loop stopped before future completed")
            
            return future.result()
        finally:
            self._running = False
            self._stopping = False

    def run_forever(self):
        if self._running:
            raise RuntimeError("Event loop is already running")
        
        self._running = True
        self._stopping = False
        
        try:
            while not self._stopping:
                self._run_once()
        finally:
            self._running = False
            self._stopping = False


async def register(loop: PollLoop, pollable) -> None:
    """Register a pollable with the event loop and wait for it to be ready."""
    future = loop.create_future()
    loop._futures[future] = pollable
    try:
        await future
    finally:
        del loop._futures[future]