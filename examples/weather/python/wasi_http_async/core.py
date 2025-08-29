"""
Core fetch implementation for WASI HTTP.
"""

import asyncio
import json as json_module
from typing import Any, Dict, Optional, Union
from urllib.parse import urlparse

from .bindings import bindings
from .poll_loop import PollLoop, register
from .stream import Stream

# Try to import Ok/Err types (may vary by componentize-py version)
try:
    from wit_world.types import Ok, Err
except ImportError:
    # Fallback if not available
    Ok = None
    Err = None


class FetchResponse:
    """Response object similar to JavaScript's fetch Response."""
    
    def __init__(self, response, body):
        self._response = response
        self._body = body  # This is the IncomingBody, not just stream
        self._body_consumed = False
        self._cached_body = None
        
    @property
    def status(self) -> int:
        """HTTP status code."""
        return self._response.status()
    
    @property
    def headers(self) -> Dict[str, str]:
        """Response headers as a dictionary."""
        headers_list = self._response.headers().entries()
        return {name: value for name, value in headers_list}
    
    @property
    def ok(self) -> bool:
        """True if status is 200-299."""
        return 200 <= self.status < 300
    
    async def text(self) -> str:
        """Read response body as text."""
        body = await self._get_body()
        return body.decode('utf-8')
    
    async def bytes(self) -> bytes:
        """Read response body as bytes."""
        return await self._get_body()
    
    async def json(self) -> Any:
        """Parse response body as JSON."""
        text = await self.text()
        return json_module.loads(text)
    
    async def _get_body(self) -> bytes:
        """Get the response body, using cache if already consumed."""
        if self._body_consumed:
            return self._cached_body or b""
        
        self._body_consumed = True
        stream = Stream(self._body)  # Pass the IncomingBody
        self._cached_body = await stream.read_all()
        return self._cached_body


async def fetch(
    url: str,
    *,
    method: str = "GET",
    headers: Optional[Dict[str, str]] = None,
    body: Optional[Union[str, bytes]] = None,
    json: Optional[Any] = None,
) -> FetchResponse:
    """
    Fetch a URL using WASI HTTP.
    
    Args:
        url: The URL to fetch
        method: HTTP method (GET, POST, etc.)
        headers: Optional headers dictionary
        body: Optional request body (string or bytes)
        json: Optional JSON body (will be serialized)
    
    Returns:
        FetchResponse object with status, headers, and body access methods
    """
    # Parse URL
    parsed = urlparse(url)
    
    # Create outgoing request
    OutgoingRequest = bindings.http_types.OutgoingRequest
    request_headers = bindings.http_types.Fields()
    
    # Add headers
    if headers:
        for name, value in headers.items():
            request_headers.append(name, value.encode() if isinstance(value, str) else value)
    
    # Handle JSON body
    if json is not None:
        body = json_module.dumps(json)
        request_headers.append("content-type", b"application/json")
    
    # Create request
    if parsed.scheme == "https":
        scheme = bindings.http_types.Scheme_Https()
    else:
        scheme = bindings.http_types.Scheme_Http()
    
    # Determine port
    default_port = 443 if parsed.scheme == "https" else 80
    authority = f"{parsed.hostname}:{parsed.port or default_port}"
    
    request = OutgoingRequest(request_headers)
    
    # Set method
    if method == "GET":
        request.set_method(bindings.http_types.Method_Get())
    elif method == "POST":
        request.set_method(bindings.http_types.Method_Post())
    elif method == "PUT":
        request.set_method(bindings.http_types.Method_Put())
    elif method == "DELETE":
        request.set_method(bindings.http_types.Method_Delete())
    elif method == "HEAD":
        request.set_method(bindings.http_types.Method_Head())
    elif method == "PATCH":
        request.set_method(bindings.http_types.Method_Patch())
    else:
        request.set_method(bindings.http_types.Method_Get())  # Default to GET
    
    request.set_scheme(scheme)
    request.set_authority(authority)
    request.set_path_with_query(parsed.path + (f"?{parsed.query}" if parsed.query else ""))
    
    # Add body if provided
    if body:
        outgoing_body = request.body()
        body_stream = outgoing_body.write()
        if isinstance(body, str):
            body = body.encode('utf-8')
        body_stream.blocking_write_and_flush(body)
        bindings.http_types.OutgoingBody.finish(outgoing_body, None)
    
    # Send request
    future = bindings.outgoing_handler.handle(request, None)
    
    # Wait for response (copy exact pattern from app.py)
    loop = asyncio.get_event_loop()
    while True:
        response = future.get()
        if response is None:
            await register(loop, future.subscribe())
        else:
            future.__exit__(None, None, None)
            
            # Handle response - simplified like app.py
            if Ok and isinstance(response, Ok):
                if isinstance(response.value, Ok):
                    incoming_response = response.value.value
                    break
                else:
                    raise Exception(f"HTTP request failed: {response.value}")
            else:
                raise Exception(f"Request failed: {response}")
    
    # Get response body (not just stream) - like app.py
    body = incoming_response.consume()
    
    return FetchResponse(incoming_response, body)


def fetch_sync(
    url: str,
    *,
    method: str = "GET",
    headers: Optional[Dict[str, str]] = None,
    body: Optional[Union[str, bytes]] = None,
    json: Optional[Any] = None,
) -> FetchResponse:
    """
    Synchronous version of fetch.
    
    Creates a temporary event loop to run the async fetch.
    """
    loop = PollLoop()
    try:
        coro = fetch(url, method=method, headers=headers, body=body, json=json)
        return loop.run_until_complete(coro)
    finally:
        loop.close()