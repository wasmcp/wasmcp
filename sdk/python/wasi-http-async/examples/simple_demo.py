#!/usr/bin/env python3
"""
Simple demonstration of all the ways to make HTTP requests with wasi-http-async.
"""

import asyncio


# Method 1: Direct fetch API (recommended)
async def method1_direct_fetch():
    from wasi_http_async import fetch
    
    response = await fetch('https://api.github.com')
    data = await response.json()
    print(f"GitHub API version: {data['current_user_url']}")


# Method 2: urllib compatibility (for legacy code)
def method2_urllib():
    import wasi_http_async
    wasi_http_async.patch_urllib()
    
    import urllib.request
    import json
    
    response = urllib.request.urlopen('https://api.github.com')
    data = json.loads(response.read())
    print(f"GitHub API version: {data['current_user_url']}")


# Method 3: Global fetch() like JavaScript
async def method3_global_fetch():
    import wasi_http_async
    wasi_http_async.install_fetch()
    
    # Now fetch() is available globally
    response = await fetch('https://api.github.com')
    data = await response.json()
    print(f"GitHub API version: {data['current_user_url']}")


# Method 4: requests compatibility (for requests users)
def method4_requests():
    from wasi_http_async import requests
    
    response = requests.get('https://api.github.com')
    data = response.json()
    print(f"GitHub API version: {data['current_user_url']}")


# Method 5: Auto-patch everything
async def method5_auto_patch():
    # This single import patches everything
    import wasi_http_async.auto_patch
    
    # Now everything works
    import urllib.request
    import json
    
    # urllib works
    response = urllib.request.urlopen('https://api.github.com')
    data = json.loads(response.read())
    print(f"urllib works: {data['current_user_url']}")
    
    # fetch() works globally
    response = await fetch('https://api.github.com')
    data = await response.json()
    print(f"fetch() works: {data['current_user_url']}")


if __name__ == "__main__":
    print("Choose your preferred HTTP method style!")
    print("All of these work in WASI components.")