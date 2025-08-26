#!/bin/bash

# Build the Python handler component to WebAssembly
echo "Building Python handler component..."

# Check if we're doing a local build (with editable SDK install)
if [ "$LOCAL_BUILD" = "true" ]; then
    echo "🔧 LOCAL BUILD MODE - Using relative paths for editable-installed SDK"
    echo "   Note: In production, wasmcp-python would be pip-installed and WIT files would be bundled"
    
    # For local development with -e installed SDK, we need explicit paths
    # The SDK is installed with pip install -e ../../../src/sdk/wasmcp-python
    SDK_DIR="../../../src/sdk/wasmcp-python"
    
    # Use the WIT file directly (not directory)
    echo "Running componentize-py with local SDK paths..."
    ../.venv/bin/componentize-py \
        -d ${SDK_DIR}/wit/handler.wit \
        -w mcp-handler \
        componentize app \
        -p ${SDK_DIR}/src \
        -p . \
        -o app.wasm
else
    # Production mode - expects wasmcp-python to be properly installed with bundled WIT
    echo "Running componentize-py in production mode..."
    echo "Note: This assumes wasmcp-python is installed with bundled WIT definitions"
    
    ../.venv/bin/componentize-py \
        componentize app \
        -p ../.venv/lib/python3.13/site-packages \
        -p . \
        -o app.wasm
fi

if [ $? -eq 0 ]; then
    echo "✅ Successfully built app.wasm"
    ls -lh app.wasm
else
    echo "❌ Build failed"
    if [ "$LOCAL_BUILD" != "true" ]; then
        echo "   Hint: If using editable-installed SDK, set LOCAL_BUILD=true"
    fi
    exit 1
fi