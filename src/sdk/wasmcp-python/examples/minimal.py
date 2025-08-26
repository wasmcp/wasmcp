#!/usr/bin/env python3
"""Minimal wasmcp handler - zero configuration required."""

from wasmcp import WasmcpHandler

handler = WasmcpHandler("minimal")

@handler.tool
def greet(name: str) -> str:
    return f"Hello, {name}!"

handler.build()

# That's it! Build with: wasmcp-build minimal.py