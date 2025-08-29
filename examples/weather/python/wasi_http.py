"""
Minimal WASI HTTP client for Python components.

This provides a simple HTTP client that works in WASM environments
by using the WASI HTTP interface directly.
"""

import json
from typing import Dict, Optional, Any

# For now, we'll use a simple approach - return mock data
# In a real implementation, this would use WASI HTTP bindings
def http_get(url: str) -> Dict[str, Any]:
    """
    Perform an HTTP GET request.
    
    NOTE: This is currently using mock data for demonstration.
    In production, this would use actual WASI HTTP bindings.
    """
    
    # Parse the URL to determine what kind of request this is
    if "geocoding-api.open-meteo.com" in url:
        # Mock geocoding response
        if "London" in url:
            return {
                "results": [{
                    "name": "London",
                    "country": "United Kingdom",
                    "latitude": 51.5074,
                    "longitude": -0.1278
                }]
            }
        elif "Paris" in url:
            return {
                "results": [{
                    "name": "Paris", 
                    "country": "France",
                    "latitude": 48.8566,
                    "longitude": 2.3522
                }]
            }
        elif "Berlin" in url:
            return {
                "results": [{
                    "name": "Berlin",
                    "country": "Germany", 
                    "latitude": 52.5200,
                    "longitude": 13.4050
                }]
            }
        else:
            return {"results": []}
            
    elif "api.open-meteo.com" in url:
        # Mock weather response
        return {
            "current": {
                "temperature_2m": 18.5,
                "apparent_temperature": 17.2,
                "relative_humidity_2m": 65,
                "wind_speed_10m": 12.3,
                "weather_code": 3  # Overcast
            }
        }
    
    raise Exception(f"Unsupported URL: {url}")