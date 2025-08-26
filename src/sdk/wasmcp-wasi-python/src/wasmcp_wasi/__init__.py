"""WASI SDK for Python MCP handlers.

This package provides WASI capabilities for MCP handlers:
- HTTP client for outbound requests
- Key-value storage (Spin-specific)  
- Configuration access
"""

from . import http
from . import config

try:
    from . import keyvalue
    HAS_KEYVALUE = True
except ImportError:
    HAS_KEYVALUE = False
    keyvalue = None  # type: ignore

__version__ = "0.1.0"

__all__ = [
    "http",
    "config",
    "keyvalue",
    "HAS_KEYVALUE",
]