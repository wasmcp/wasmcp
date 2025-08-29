# wasi-http-async

Simple async HTTP client for WebAssembly components using WASI.

## Features

- Simple fetch-like API for HTTP requests in WASI components
- Both async and sync interfaces
- Automatic resource management
- Dynamic discovery of WASI bindings
- Compatible with componentize-py
- Multiple usage patterns: direct import, urllib patching, requests compatibility

## Installation

```bash
pip install wasi-http-async
```

## Quick Start

### Basic Usage

```python
from wasi_http_async import fetch, fetch_sync

# Async
async def get_data():
    response = await fetch('https://api.example.com/data')
    return await response.json()

# Sync
def get_data_sync():
    response = fetch_sync('https://api.example.com/data')
    return response.json()
```

### urllib Compatibility

```python
import wasi_http_async
wasi_http_async.patch_urllib()

import urllib.request
response = urllib.request.urlopen('https://api.example.com/data')
data = response.read()
```

### requests Compatibility

```python
from wasi_http_async import requests

response = requests.get('https://api.example.com/data')
data = response.json()
```

## How It Works

This library provides a thin wrapper around WASI HTTP interfaces, handling:
- Async event loop backed by wasi:io/poll
- Automatic resource cleanup
- Stream handling for response bodies
- Dynamic discovery of componentize-py generated bindings

## Requirements

- Python 3.10+
- componentize-py
- WASI-enabled runtime (wasmtime, etc.)

## License

MIT