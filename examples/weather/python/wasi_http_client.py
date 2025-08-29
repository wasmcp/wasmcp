"""
WASI HTTP client for Python components.

This module provides a simple HTTP client that uses the WASI HTTP interface
to make outbound HTTP requests from a WebAssembly component.
"""

import json
from typing import Dict, Any
from urllib.parse import urlparse


def http_get(url: str) -> Dict[str, Any]:
    """
    Perform an HTTP GET request using WASI HTTP.
    
    Args:
        url: The URL to fetch
        
    Returns:
        The parsed JSON response
    """
    # Import inside function to avoid issues during componentize-py build
    from wit_world.imports import http_types, outgoing_handler, io_streams, io_poll
    
    parsed = urlparse(url)
    
    # Create headers
    headers = http_types.Fields()
    
    # Create the outgoing request
    request = http_types.OutgoingRequest(headers)
    
    # Set the method
    request.set_method(http_types.Method_Get())
    
    # Set the scheme
    if parsed.scheme == "https":
        request.set_scheme(http_types.Scheme_Https())
    else:
        request.set_scheme(http_types.Scheme_Http())
    
    # Set the authority (host:port)
    authority = parsed.netloc
    if not authority:
        if parsed.scheme == "https":
            authority = parsed.hostname + ":443"
        else:
            authority = parsed.hostname + ":80"
    request.set_authority(authority)
    
    # Set the path and query
    path_with_query = parsed.path or "/"
    if parsed.query:
        path_with_query += "?" + parsed.query
    request.set_path_with_query(path_with_query)
    
    # Send the request
    future_response = outgoing_handler.handle(request, None)
    
    # Poll until response is ready
    pollable = future_response.subscribe()
    io_poll.poll([pollable])
    
    # Get the response
    response_result = future_response.get()
    
    if response_result is None:
        raise Exception("No response received")
    
    # Unwrap the result
    if isinstance(response_result, http_types.Result):
        if response_result.is_err():
            raise Exception(f"HTTP request failed: {response_result.err()}")
        incoming_response = response_result.ok()
    else:
        incoming_response = response_result
    
    # Get the status code
    status = incoming_response.status()
    
    if status != 200:
        raise Exception(f"HTTP request failed with status {status}")
    
    # Read the response body
    body = incoming_response.consume()
    stream = body.stream()
    
    response_data = bytearray()
    while True:
        # Try to read data
        data_result = stream.read(8192)
        if data_result is None:
            break
        if isinstance(data_result, http_types.Result):
            if data_result.is_err():
                break
            chunk = data_result.ok()
        else:
            chunk = data_result
        
        if not chunk or len(chunk) == 0:
            break
        response_data.extend(chunk)
    
    # Parse the JSON response
    return json.loads(response_data.decode('utf-8'))