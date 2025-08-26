#!/bin/bash

# Build the Python handler component to WebAssembly
echo "Building Python handler component..."

# Uninstall Spin SDK which has conflicting WIT files
echo "Uninstalling spin-sdk to remove conflicting WIT files..."
../.venv/bin/pip uninstall -y spin-sdk

# Now try the original command
echo ""
echo "Trying build without Spin SDK interference..."
../.venv/bin/componentize-py \
    -d mcp.wit \
    -w mcp-handler \
    componentize app \
    -p ../.venv/lib/python3.13/site-packages \
    -p ../../../src/sdk/wasmcp-python/src \
    -p . \
    -o app.wasm

if [ $? -eq 0 ]; then
    echo "✅ Successfully built app.wasm"
    ls -lh app.wasm
else
    echo "❌ Build failed even without Spin SDK"
fi