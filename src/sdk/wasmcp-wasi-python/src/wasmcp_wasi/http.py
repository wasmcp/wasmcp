"""HTTP client for WASI environments using direct WASI bindings."""

import json
from typing import Any, Dict, Optional, Union
from enum import Enum

# Note: In a real WASI environment, these would be imported from the WASI bindings
# For now, we'll create stub implementations that match the expected interface
try:
    # Try to import WASI bindings if available
    import urllib.request
    import urllib.parse
    import urllib.error
    HTTP_AVAILABLE = True
except ImportError:
    HTTP_AVAILABLE = False


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
        RuntimeError: If HTTP is not available in WASI environment
    """
    if not HTTP_AVAILABLE:
        raise RuntimeError("HTTP not available in this environment")
    
    # In a real WASI environment, this would use the WASI HTTP bindings
    # For testing/development, we'll use a stub that simulates the behavior
    wasi_request = request.to_wasi_request()
    
    # Create urllib request
    urllib_request = urllib.request.Request(
        wasi_request["uri"],
        data=wasi_request["body"],
        method=wasi_request["method"]
    )
    
    # Add headers
    for name, value in wasi_request["headers"]:
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