"""
requests-compatible API for WASI HTTP.

Provides a drop-in replacement for common requests library patterns.
"""

import json as json_module
from typing import Any, Dict, Optional, Union
from urllib.parse import urlencode

from .core import fetch_sync


class Response:
    """requests-compatible Response object."""
    
    def __init__(self, fetch_response, url):
        self._fetch_response = fetch_response
        self.url = url
        self.status_code = fetch_response.status
        self.headers = fetch_response.headers
        self.ok = fetch_response.ok
        self._content = None
        self._text = None
        
    @property
    def content(self):
        """Get response body as bytes."""
        if self._content is None:
            # Get body synchronously
            import asyncio
            from .poll_loop import PollLoop
            
            loop = PollLoop()
            try:
                coro = self._fetch_response.bytes()
                self._content = loop.run_until_complete(coro)
            finally:
                loop.close()
        return self._content
    
    @property
    def text(self):
        """Get response body as text."""
        if self._text is None:
            self._text = self.content.decode('utf-8')
        return self._text
    
    def json(self):
        """Parse response body as JSON."""
        return json_module.loads(self.text)
    
    def raise_for_status(self):
        """Raise an exception for bad status codes."""
        if not self.ok:
            raise Exception(f"HTTP {self.status_code} Error for URL: {self.url}")


class Session:
    """requests-compatible Session object."""
    
    def __init__(self):
        self.headers = {}
        self.cookies = {}
        self.auth = None
        self.proxies = {}
        self.verify = True
        self.cert = None
        
    def request(
        self,
        method: str,
        url: str,
        params: Optional[Dict[str, Any]] = None,
        data: Optional[Union[str, bytes, Dict]] = None,
        json: Optional[Any] = None,
        headers: Optional[Dict[str, str]] = None,
        cookies: Optional[Dict[str, str]] = None,
        files: Optional[Dict] = None,
        auth: Optional[Any] = None,
        timeout: Optional[float] = None,
        allow_redirects: bool = True,
        proxies: Optional[Dict] = None,
        hooks: Optional[Dict] = None,
        stream: bool = False,
        verify: bool = True,
        cert: Optional[Any] = None,
    ) -> Response:
        """Make an HTTP request."""
        
        # Build URL with params
        if params:
            url = f"{url}?{urlencode(params)}"
        
        # Merge headers
        req_headers = dict(self.headers)
        if headers:
            req_headers.update(headers)
        
        # Handle form data
        body = None
        if data:
            if isinstance(data, dict):
                body = urlencode(data)
                req_headers['Content-Type'] = 'application/x-www-form-urlencoded'
            else:
                body = data
        
        # Make request
        fetch_response = fetch_sync(
            url,
            method=method,
            headers=req_headers,
            body=body,
            json=json
        )
        
        return Response(fetch_response, url)
    
    def get(self, url, **kwargs):
        """Make a GET request."""
        return self.request('GET', url, **kwargs)
    
    def post(self, url, data=None, json=None, **kwargs):
        """Make a POST request."""
        return self.request('POST', url, data=data, json=json, **kwargs)
    
    def put(self, url, data=None, **kwargs):
        """Make a PUT request."""
        return self.request('PUT', url, data=data, **kwargs)
    
    def delete(self, url, **kwargs):
        """Make a DELETE request."""
        return self.request('DELETE', url, **kwargs)
    
    def head(self, url, **kwargs):
        """Make a HEAD request."""
        return self.request('HEAD', url, **kwargs)
    
    def patch(self, url, data=None, **kwargs):
        """Make a PATCH request."""
        return self.request('PATCH', url, data=data, **kwargs)
    
    def close(self):
        """Close the session."""
        pass


# Module-level convenience functions
_default_session = Session()


def request(method: str, url: str, **kwargs) -> Response:
    """Make an HTTP request."""
    return _default_session.request(method, url, **kwargs)


def get(url: str, params=None, **kwargs) -> Response:
    """Make a GET request."""
    return request('GET', url, params=params, **kwargs)


def post(url: str, data=None, json=None, **kwargs) -> Response:
    """Make a POST request."""
    return request('POST', url, data=data, json=json, **kwargs)


def put(url: str, data=None, **kwargs) -> Response:
    """Make a PUT request."""
    return request('PUT', url, data=data, **kwargs)


def delete(url: str, **kwargs) -> Response:
    """Make a DELETE request."""
    return request('DELETE', url, **kwargs)


def head(url: str, **kwargs) -> Response:
    """Make a HEAD request."""
    return request('HEAD', url, **kwargs)


def patch(url: str, data=None, **kwargs) -> Response:
    """Make a PATCH request."""
    return request('PATCH', url, data=data, **kwargs)


# Common exceptions for compatibility
class RequestException(Exception):
    """Base exception for requests."""
    pass


class HTTPError(RequestException):
    """HTTP error exception."""
    pass


class ConnectionError(RequestException):
    """Connection error exception."""
    pass


class Timeout(RequestException):
    """Timeout exception."""
    pass


class TooManyRedirects(RequestException):
    """Too many redirects exception."""
    pass