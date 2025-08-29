"""
Compatibility layers for standard Python HTTP libraries.
"""

import builtins
import io
import sys
import urllib.request
import urllib.parse
import urllib.error
from typing import Any, Dict, Optional
from http.client import HTTPResponse, HTTPMessage
from email.message import Message

from .core import fetch_sync


class WASIHTTPResponse:
    """urllib-compatible response wrapper."""
    
    def __init__(self, fetch_response):
        self._fetch_response = fetch_response
        self._body = None
        self.code = self._fetch_response.status
        self.msg = "OK" if 200 <= self.code < 300 else "Error"
        self.headers = self._create_headers()
        self.url = None  # Set by opener
        
    def _create_headers(self):
        """Create HTTPMessage-compatible headers."""
        msg = Message()
        for name, value in self._fetch_response.headers.items():
            msg[name] = value
        return msg
    
    def read(self, amt=None):
        """Read response body."""
        if self._body is None:
            # Fetch sync response - need to get body synchronously
            import asyncio
            from .poll_loop import PollLoop
            
            loop = PollLoop()
            try:
                coro = self._fetch_response.bytes()
                self._body = loop.run_until_complete(coro)
            finally:
                loop.close()
        
        if amt is None:
            return self._body
        return self._body[:amt]
    
    def getcode(self):
        """Get HTTP status code."""
        return self.code
    
    def geturl(self):
        """Get the URL."""
        return self.url
    
    def info(self):
        """Get headers."""
        return self.headers
    
    def close(self):
        """Close the response."""
        pass


class WASIHTTPHandler(urllib.request.HTTPHandler):
    """urllib HTTP handler using WASI HTTP."""
    
    def http_open(self, req):
        return self._do_open(req)
    
    def _do_open(self, req):
        """Open HTTP connection using WASI fetch."""
        url = req.get_full_url()
        method = req.get_method()
        headers = dict(req.headers)
        
        # Get request body if present
        body = None
        if req.data:
            body = req.data
            if isinstance(body, str):
                body = body.encode('utf-8')
        
        # Make request
        fetch_response = fetch_sync(
            url,
            method=method,
            headers=headers,
            body=body
        )
        
        # Create urllib-compatible response
        response = WASIHTTPResponse(fetch_response)
        response.url = url
        
        return response


class WASIHTTPSHandler(WASIHTTPHandler, urllib.request.HTTPSHandler):
    """urllib HTTPS handler using WASI HTTP."""
    
    def https_open(self, req):
        return self._do_open(req)


def patch_urllib():
    """
    Patch urllib to use WASI HTTP for all requests.
    
    After calling this function, urllib.request.urlopen() and related
    functions will use WASI HTTP instead of the standard socket-based
    implementation.
    """
    # Replace default handlers
    opener = urllib.request.build_opener(
        WASIHTTPHandler(),
        WASIHTTPSHandler()
    )
    urllib.request.install_opener(opener)
    
    # Also patch http.client for completeness
    import http.client
    original_HTTPConnection = http.client.HTTPConnection
    original_HTTPSConnection = http.client.HTTPSConnection
    
    class WASIHTTPConnection:
        """Minimal HTTPConnection replacement using WASI fetch."""
        
        def __init__(self, host, port=None, *args, **kwargs):
            self.host = host
            self.port = port or 80
            self._response = None
            
        def request(self, method, url, body=None, headers=None):
            full_url = f"http://{self.host}:{self.port}{url}"
            self._response = fetch_sync(
                full_url,
                method=method,
                headers=dict(headers or {}),
                body=body
            )
        
        def getresponse(self):
            return WASIHTTPResponse(self._response)
        
        def close(self):
            pass
    
    class WASIHTTPSConnection(WASIHTTPConnection):
        def __init__(self, host, port=None, *args, **kwargs):
            super().__init__(host, port or 443, *args, **kwargs)
            
        def request(self, method, url, body=None, headers=None):
            full_url = f"https://{self.host}:{self.port}{url}"
            self._response = fetch_sync(
                full_url,
                method=method,
                headers=dict(headers or {}),
                body=body
            )
    
    http.client.HTTPConnection = WASIHTTPConnection
    http.client.HTTPSConnection = WASIHTTPSConnection


def install_fetch():
    """
    Install a global fetch() function similar to JavaScript's fetch.
    
    After calling this function, fetch() will be available in the global
    namespace and can be used like:
    
        response = await fetch('https://api.example.com/data')
        data = await response.json()
    """
    from .core import fetch
    
    # Add to builtins so it's available everywhere
    builtins.fetch = fetch
    
    # Also add to main module globals
    if hasattr(sys, 'modules'):
        main = sys.modules.get('__main__')
        if main:
            setattr(main, 'fetch', fetch)