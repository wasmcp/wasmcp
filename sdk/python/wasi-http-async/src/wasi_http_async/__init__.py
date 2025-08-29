"""
wasi-http-async: Simple async HTTP client for WebAssembly components using WASI
"""

from .core import fetch, fetch_sync, FetchResponse
from .compat import patch_urllib, install_fetch
from . import requests

__version__ = "0.1.0"
__all__ = [
    "fetch",
    "fetch_sync", 
    "FetchResponse",
    "patch_urllib",
    "install_fetch",
    "requests",
]