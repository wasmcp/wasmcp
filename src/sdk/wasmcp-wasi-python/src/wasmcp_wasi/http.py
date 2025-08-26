"""HTTP client for WASI environments using direct WASI bindings."""

import json
from typing import Any, Dict, List, Optional, Union, Tuple
from enum import Enum


class HttpMethod(Enum):
    """HTTP methods."""
    GET = "GET"
    POST = "POST"
    PUT = "PUT"
    DELETE = "DELETE"
    PATCH = "PATCH"
    HEAD = "HEAD"
    OPTIONS = "OPTIONS"


class Request:
    """HTTP request builder."""
    
    def __init__(
        self,
        url: str,
        method: Union[HttpMethod, str] = HttpMethod.GET,
        headers: Optional[Dict[str, str]] = None,
        body: Optional[Union[str, bytes, dict]] = None
    ):
        """Initialize a request.
        
        Args:
            url: Target URL
            method: HTTP method
            headers: Request headers
            body: Request body (string, bytes, or dict to be JSON-encoded)
        """
        self.url = url
        self.method = HttpMethod(method) if isinstance(method, str) else method
        self.headers = headers or {}
        self.body = body
    
    def _prepare_body(self) -> Optional[bytes]:
        """Prepare the body for sending.
        
        Returns:
            Body as bytes or None
        """
        if self.body is None:
            return None
        
        if isinstance(self.body, bytes):
            return self.body
        elif isinstance(self.body, str):
            return self.body.encode('utf-8')
        else:
            # Assume dict or other JSON-serializable
            json_str = json.dumps(self.body)
            if 'Content-Type' not in self.headers:
                self.headers['Content-Type'] = 'application/json'
            return json_str.encode('utf-8')
    
    def to_wasi_request(self) -> Dict[str, Any]:
        """Convert to WASI request format.
        
        Returns:
            Dict representing WASI request
        """
        body = self._prepare_body()
        
        return {
            "method": self.method.value,
            "uri": self.url,
            "headers": list(self.headers.items()),
            "body": body
        }


class Response:
    """HTTP response wrapper."""
    
    def __init__(self, status: int, headers: Dict[str, str], body: bytes):
        """Initialize response.
        
        Args:
            status: HTTP status code
            headers: Response headers
            body: Response body as bytes
        """
        self._status = status
        self._headers = headers
        self._body = body
        self._text: Optional[str] = None
        self._json: Optional[Any] = None
    
    @property
    def status(self) -> int:
        """Get response status code.
        
        Returns:
            HTTP status code
        """
        return self._status
    
    @property
    def headers(self) -> Dict[str, str]:
        """Get response headers.
        
        Returns:
            Dictionary of headers
        """
        return self._headers
    
    @property
    def body(self) -> bytes:
        """Get response body as bytes.
        
        Returns:
            Response body
        """
        return self._body
    
    def text(self) -> str:
        """Get response body as text.
        
        Returns:
            Response body as string
        """
        if self._text is None:
            self._text = self._body.decode('utf-8')
        return self._text
    
    def json(self) -> Any:
        """Parse response body as JSON.
        
        Returns:
            Parsed JSON data
            
        Raises:
            json.JSONDecodeError: If body is not valid JSON
        """
        if self._json is None:
            self._json = json.loads(self.text())
        return self._json
    
    @property
    def ok(self) -> bool:
        """Check if response is successful (2xx status).
        
        Returns:
            True if status is 2xx, False otherwise
        """
        return 200 <= self._status < 300


def send(request: Request) -> Response:
    """Send HTTP request using WASI bindings.
    
    Args:
        request: Request to send
        
    Returns:
        Response object
        
    Raises:
        RuntimeError: If HTTP request fails
    """
    try:
        # Import WASI HTTP bindings
        from .bindings.wasi_http import request as wasi_request
        
        # Convert to WASI format
        wasi_req = request.to_wasi_request()
        
        # Make WASI HTTP request
        wasi_response = wasi_request(
            method=wasi_req["method"],
            uri=wasi_req["uri"],
            headers=wasi_req["headers"],
            body=wasi_req["body"]
        )
        
        return Response(
            status=wasi_response["status"],
            headers=dict(wasi_response["headers"]),
            body=wasi_response["body"]
        )
        
    except ImportError:
        # WASI bindings not available - try componentize-py bindings
        try:
            from .bindings.http import outgoing_request, outgoing_body
            from .bindings.http import method as http_method
            from .bindings.http import scheme as http_scheme
            from .bindings.http import fields as http_fields
            
            # Create outgoing request
            method = getattr(http_method, request.method.value.lower())
            
            # Parse URL
            if request.url.startswith('https://'):
                scheme = http_scheme.https
                url_without_scheme = request.url[8:]
            elif request.url.startswith('http://'):
                scheme = http_scheme.http
                url_without_scheme = request.url[7:]
            else:
                raise ValueError(f"Unsupported URL scheme: {request.url}")
            
            if '/' in url_without_scheme:
                authority, path_and_query = url_without_scheme.split('/', 1)
                path_and_query = '/' + path_and_query
            else:
                authority = url_without_scheme
                path_and_query = '/'
            
            # Create headers
            headers = http_fields.Fields()
            for name, value in request.headers.items():
                headers.append(name.lower(), value.encode('utf-8'))
            
            # Create request
            req = outgoing_request.OutgoingRequest(
                method=method,
                path_with_query=path_and_query,
                scheme=scheme,
                authority=authority,
                headers=headers
            )
            
            # Add body if present
            body = request._prepare_body()
            if body:
                outgoing_body_obj = req.body()
                stream = outgoing_body_obj.write()
                stream.write(body)
                stream.flush()
                outgoing_body.finish(outgoing_body_obj)
            
            # Send request and get response
            future_response = req.send()
            incoming_response = future_response.get()
            
            # Extract response data
            status = incoming_response.status()
            response_headers = {}
            for name, values in incoming_response.headers().entries():
                if values:
                    response_headers[name] = values[0].decode('utf-8')
            
            # Read response body
            response_body = b''
            if hasattr(incoming_response, 'consume'):
                body_stream = incoming_response.consume()
                while True:
                    chunk = body_stream.read(8192)
                    if not chunk:
                        break
                    response_body += chunk
            
            return Response(status, response_headers, response_body)
            
        except ImportError:
            # Fallback to direct WASI system calls
            try:
                import wasmtime
                
                # This would use direct WASI system calls in a real WASI environment
                # For now, we'll raise an error since we need actual WASI HTTP support
                raise RuntimeError(
                    "WASI HTTP bindings not available. "
                    "This implementation requires running in a WASI environment "
                    "with HTTP capabilities enabled."
                )
                
            except ImportError:
                # Last resort - use urllib for development/testing ONLY
                import urllib.request
                import urllib.error
                
                # Create urllib request
                urllib_request = urllib.request.Request(
                    request.url,
                    data=request._prepare_body(),
                    method=request.method.value
                )
                
                # Add headers
                for name, value in request.headers.items():
                    urllib_request.add_header(name, value)
                
                try:
                    response = urllib.request.urlopen(urllib_request)
                    status = response.getcode()
                    headers = dict(response.headers)
                    body = response.read()
                    return Response(status, headers, body)
                except urllib.error.HTTPError as e:
                    # Return error response
                    headers = dict(e.headers) if e.headers else {}
                    body = e.read() if hasattr(e, 'read') else b""
                    return Response(e.code, headers, body)


def get(url: str, headers: Optional[Dict[str, str]] = None) -> Response:
    """Send GET request.
    
    Args:
        url: Target URL
        headers: Optional headers
        
    Returns:
        Response object
    """
    request = Request(url, HttpMethod.GET, headers=headers)
    return send(request)


def post(
    url: str,
    headers: Optional[Dict[str, str]] = None,
    body: Optional[Union[str, bytes, dict]] = None
) -> Response:
    """Send POST request.
    
    Args:
        url: Target URL
        headers: Optional headers
        body: Request body
        
    Returns:
        Response object
    """
    request = Request(url, HttpMethod.POST, headers=headers, body=body)
    return send(request)


def put(
    url: str,
    headers: Optional[Dict[str, str]] = None,
    body: Optional[Union[str, bytes, dict]] = None
) -> Response:
    """Send PUT request.
    
    Args:
        url: Target URL
        headers: Optional headers
        body: Request body
        
    Returns:
        Response object
    """
    request = Request(url, HttpMethod.PUT, headers=headers, body=body)
    return send(request)


def delete(url: str, headers: Optional[Dict[str, str]] = None) -> Response:
    """Send DELETE request.
    
    Args:
        url: Target URL
        headers: Optional headers
        
    Returns:
        Response object
    """
    request = Request(url, HttpMethod.DELETE, headers=headers)
    return send(request)


def patch(
    url: str,
    headers: Optional[Dict[str, str]] = None,
    body: Optional[Union[str, bytes, dict]] = None
) -> Response:
    """Send PATCH request.
    
    Args:
        url: Target URL
        headers: Optional headers
        body: Request body
        
    Returns:
        Response object
    """
    request = Request(url, HttpMethod.PATCH, headers=headers, body=body)
    return send(request)