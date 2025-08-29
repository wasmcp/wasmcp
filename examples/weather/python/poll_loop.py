"""
Componentize-py's poll_loop adapted for our tools-handler world.
Based on the official componentize-py bundled/poll_loop.py
"""

import asyncio
from typing import Optional, cast

from wit_world.types import Ok, Err
from wit_world.imports import wasi_http_types as types, streams, poll, outgoing_handler
from wit_world.imports.wasi_http_types import (
    IncomingBody,
    OutgoingBody,
    OutgoingRequest,
    IncomingResponse,
)
from wit_world.imports.streams import StreamError_Closed, InputStream
from wit_world.imports.poll import Pollable

# Maximum number of bytes to read at a time
READ_SIZE: int = 16 * 1024


async def send(request: OutgoingRequest) -> IncomingResponse:
    """Send the specified request and wait asynchronously for the response."""
    future = outgoing_handler.handle(request, None)

    while True:
        response = future.get()
        if response is None:
            await register(cast(PollLoop, asyncio.get_event_loop()), future.subscribe())
        else:
            if isinstance(response, Ok):
                if isinstance(response.value, Ok):
                    return response.value.value
                else:
                    raise response.value
            else:
                raise response


class Stream:
    """Reader abstraction over `wasi:http/types#incoming-body`."""

    def __init__(self, body: IncomingBody):
        self.body: Optional[IncomingBody] = body
        self.stream: Optional[InputStream] = body.stream()

    async def next(self) -> Optional[bytes]:
        """Wait for the next chunk of data to arrive on the stream.
        
        This will return `None` when the end of the stream has been reached.
        """
        while True:
            try:
                if self.stream is None:
                    return None
                else:
                    buffer = self.stream.read(READ_SIZE)
                    if len(buffer) == 0:
                        await register(
                            cast(PollLoop, asyncio.get_event_loop()),
                            self.stream.subscribe(),
                        )
                    else:
                        return buffer
            except Err as e:
                if isinstance(e.value, StreamError_Closed):
                    if self.stream is not None:
                        self.stream.__exit__(None, None, None)
                        self.stream = None
                    if self.body is not None:
                        IncomingBody.finish(self.body)
                        self.body = None
                else:
                    raise e


class PollLoop(asyncio.AbstractEventLoop):
    """Custom `asyncio` event loop backed by `wasi:io/poll#poll`."""

    def __init__(self):
        self.wakers = []
        self.running = False
        self.handles = []
        self.exception = None

    def get_debug(self):
        return False

    def run_until_complete(self, future):
        future = asyncio.ensure_future(future, loop=self)

        self.running = True
        asyncio.events._set_running_loop(self)
        while self.running and not future.done():
            handles = self.handles
            self.handles = []
            for handle in handles:
                if not handle._cancelled:
                    handle._run()

            if self.wakers:
                [pollables, wakers] = list(map(list, zip(*self.wakers)))

                new_wakers = []
                ready = [False] * len(pollables)
                for index in poll.poll(pollables):
                    ready[index] = True

                for (ready, pollable), waker in zip(zip(ready, pollables), wakers):
                    if ready:
                        pollable.__exit__(None, None, None)
                        waker.set_result(None)
                    else:
                        new_wakers.append((pollable, waker))

                self.wakers = new_wakers

            if self.exception is not None:
                raise self.exception

        return future.result()

    def is_running(self):
        return self.running

    def is_closed(self):
        return not self.running

    def stop(self):
        self.running = False

    def close(self):
        self.running = False

    def shutdown_asyncgens(self):
        pass

    def call_exception_handler(self, context):
        self.exception = context.get("exception", None)

    def call_soon(self, callback, *args, context=None):
        handle = asyncio.Handle(callback, args, self, context)
        self.handles.append(handle)
        return handle

    def create_task(self, coroutine):
        return asyncio.Task(coroutine, loop=self)

    def create_future(self):
        return asyncio.Future(loop=self)

    # The remaining methods are not implemented for WASI
    def run_forever(self):
        raise NotImplementedError

    async def shutdown_default_executor(self):
        raise NotImplementedError

    def _timer_handle_cancelled(self, handle):
        raise NotImplementedError

    def call_later(self, delay, callback, *args, context=None):
        raise NotImplementedError

    def call_at(self, when, callback, *args, context=None):
        raise NotImplementedError

    def time(self):
        raise NotImplementedError

    def call_soon_threadsafe(self, callback, *args, context=None):
        raise NotImplementedError

    def run_in_executor(self, executor, func, *args):
        raise NotImplementedError

    def set_default_executor(self, executor):
        raise NotImplementedError

    async def getaddrinfo(self, host, port, *, family=0, type=0, proto=0, flags=0):
        raise NotImplementedError

    async def getnameinfo(self, sockaddr, flags=0):
        raise NotImplementedError

    async def create_connection(self, *args, **kwargs):
        raise NotImplementedError

    async def create_server(self, *args, **kwargs):
        raise NotImplementedError

    async def sendfile(self, *args, **kwargs):
        raise NotImplementedError

    async def start_tls(self, *args, **kwargs):
        raise NotImplementedError

    async def create_unix_connection(self, *args, **kwargs):
        raise NotImplementedError

    async def create_unix_server(self, *args, **kwargs):
        raise NotImplementedError

    async def connect_accepted_socket(self, *args, **kwargs):
        raise NotImplementedError

    async def create_datagram_endpoint(self, *args, **kwargs):
        raise NotImplementedError

    async def connect_read_pipe(self, protocol_factory, pipe):
        raise NotImplementedError

    async def connect_write_pipe(self, protocol_factory, pipe):
        raise NotImplementedError

    async def subprocess_shell(self, *args, **kwargs):
        raise NotImplementedError

    async def subprocess_exec(self, *args, **kwargs):
        raise NotImplementedError

    def add_reader(self, fd, callback, *args):
        raise NotImplementedError

    def remove_reader(self, fd):
        raise NotImplementedError

    def add_writer(self, fd, callback, *args):
        raise NotImplementedError

    def remove_writer(self, fd):
        raise NotImplementedError

    async def sock_recv(self, sock, nbytes):
        raise NotImplementedError

    async def sock_recv_into(self, sock, buf):
        raise NotImplementedError

    async def sock_recvfrom(self, sock, bufsize):
        raise NotImplementedError

    async def sock_recvfrom_into(self, sock, buf, nbytes=0):
        raise NotImplementedError

    async def sock_sendall(self, sock, data):
        raise NotImplementedError

    async def sock_sendto(self, sock, data, address):
        raise NotImplementedError

    async def sock_connect(self, sock, address):
        raise NotImplementedError

    async def sock_accept(self, sock):
        raise NotImplementedError

    async def sock_sendfile(self, sock, file, offset=0, count=None, *, fallback=None):
        raise NotImplementedError

    def add_signal_handler(self, sig, callback, *args):
        raise NotImplementedError

    def remove_signal_handler(self, sig):
        raise NotImplementedError

    def set_task_factory(self, factory):
        raise NotImplementedError

    def get_task_factory(self):
        raise NotImplementedError

    def get_exception_handler(self):
        raise NotImplementedError

    def set_exception_handler(self, handler):
        raise NotImplementedError

    def default_exception_handler(self, context):
        raise NotImplementedError

    def set_debug(self, enabled):
        raise NotImplementedError


async def register(loop: PollLoop, pollable: Pollable):
    waker = loop.create_future()
    loop.wakers.append((pollable, waker))
    await waker