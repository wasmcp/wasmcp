#!/bin/bash
set -e

echo "Building minimal_app with mcp world from local WIT and spin_sdk for HTTP..."
.venv/bin/componentize-py componentize minimal_app.app -p . -p .venv/lib/python3.11/site-packages -m spin_sdk=spin-imports -o minimal.wasm

echo "Build complete: minimal.wasm"
ls -lah minimal.wasm