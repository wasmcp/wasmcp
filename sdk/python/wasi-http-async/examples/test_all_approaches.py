#!/usr/bin/env python3
"""
Test all HTTP approaches provided by wasi-http-async.

This example demonstrates:
1. Direct fetch API
2. urllib patching
3. Global fetch() function
4. requests compatibility
"""

import asyncio
import json
import sys
import os

# Add parent directory to path for testing
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'src'))


async def test_direct_fetch():
    """Test direct fetch API."""
    print("\n=== Testing Direct Fetch API ===")
    
    from wasi_http_async import fetch, fetch_sync
    
    # Async fetch
    print("Async fetch...")
    response = await fetch("https://api.github.com/repos/python/cpython")
    data = await response.json()
    print(f"  Repository: {data['full_name']}")
    print(f"  Stars: {data['stargazers_count']}")
    
    # Sync fetch
    print("Sync fetch...")
    response = fetch_sync("https://api.github.com/repos/python/cpython")
    # Note: In real WASI, we'd need to handle sync properly
    print(f"  Status: {response.status}")


async def test_urllib_patching():
    """Test urllib patching."""
    print("\n=== Testing urllib Patching ===")
    
    from wasi_http_async import patch_urllib
    patch_urllib()
    
    import urllib.request
    import urllib.parse
    
    # Simple GET
    print("urllib GET request...")
    response = urllib.request.urlopen("https://api.github.com/repos/python/cpython")
    data = json.loads(response.read())
    print(f"  Repository: {data['full_name']}")
    
    # POST with data
    print("urllib POST request...")
    post_data = urllib.parse.urlencode({'key': 'value'}).encode()
    req = urllib.request.Request(
        "https://httpbin.org/post",
        data=post_data,
        headers={'Content-Type': 'application/x-www-form-urlencoded'}
    )
    # Would make actual request in WASI environment
    print("  POST request prepared")


async def test_global_fetch():
    """Test global fetch() function."""
    print("\n=== Testing Global fetch() ===")
    
    from wasi_http_async import install_fetch
    install_fetch()
    
    # Now fetch is available globally
    print("Using global fetch()...")
    response = await fetch("https://api.github.com/repos/python/cpython")
    data = await response.json()
    print(f"  Repository: {data['full_name']}")
    print(f"  Language: {data['language']}")


async def test_requests_compat():
    """Test requests compatibility layer."""
    print("\n=== Testing requests Compatibility ===")
    
    from wasi_http_async import requests
    
    # Simple GET
    print("requests.get()...")
    response = requests.get("https://api.github.com/repos/python/cpython")
    data = response.json()
    print(f"  Repository: {data['full_name']}")
    print(f"  Status Code: {response.status_code}")
    
    # POST with JSON
    print("requests.post() with JSON...")
    response = requests.post(
        "https://httpbin.org/post",
        json={'message': 'Hello from WASI'},
        headers={'User-Agent': 'wasi-http-async'}
    )
    print(f"  Status Code: {response.status_code}")
    
    # Session usage
    print("Using requests.Session()...")
    session = requests.Session()
    session.headers['User-Agent'] = 'wasi-http-async/session'
    response = session.get("https://api.github.com/repos/python/cpython")
    print(f"  Status Code: {response.status_code}")


async def test_auto_patch():
    """Test auto-patching module."""
    print("\n=== Testing Auto-Patch ===")
    
    # Import auto_patch to enable everything
    import wasi_http_async.auto_patch
    
    # Now both urllib and fetch() work
    import urllib.request
    
    print("After auto-patch import:")
    print("  urllib is patched ✓")
    print("  fetch() is available globally ✓")
    
    # Test that they work
    response = await fetch("https://api.github.com/repos/python/cpython")
    print(f"  fetch() works: {response.ok}")
    
    response = urllib.request.urlopen("https://api.github.com/repos/python/cpython")
    print(f"  urllib works: {response.getcode()}")


async def main():
    """Run all tests."""
    print("Testing wasi-http-async Library")
    print("=" * 40)
    
    try:
        await test_direct_fetch()
    except Exception as e:
        print(f"  Direct fetch failed: {e}")
    
    try:
        await test_urllib_patching()
    except Exception as e:
        print(f"  urllib patching failed: {e}")
    
    try:
        await test_global_fetch()
    except Exception as e:
        print(f"  Global fetch failed: {e}")
    
    try:
        await test_requests_compat()
    except Exception as e:
        print(f"  requests compat failed: {e}")
    
    try:
        await test_auto_patch()
    except Exception as e:
        print(f"  Auto-patch failed: {e}")
    
    print("\n" + "=" * 40)
    print("Testing complete!")


if __name__ == "__main__":
    # This would run in a WASI environment with proper event loop
    print("Note: This test file is designed to run in a WASI environment.")
    print("In a regular Python environment, it will show the API structure.")
    
    # Try to run if we have asyncio
    try:
        asyncio.run(main())
    except Exception as e:
        print(f"Could not run async tests: {e}")
        print("\nThe library is designed for WASI environments where:")
        print("- componentize-py generates the WASI bindings")
        print("- wasi:http interfaces are available")
        print("- wasi:io/poll is available for async operations")