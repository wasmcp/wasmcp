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
    
    # Step 1: Generate bindings from WIT files
    echo "📝 Generating Python bindings from WIT files..."
    rm -rf bindings  # Clean old bindings
    
    # Generate bindings using correct syntax
    # Point to the wit directory which contains both handler.wit and deps
    # Use fully qualified world name to avoid ambiguity with Spin worlds
    ../.venv/bin/componentize-py \
        -d ${SDK_DIR}/wit \
        -w wasmcp:mcp/mcp-handler \
        bindings bindings
    
    if [ $? -ne 0 ]; then
        echo "❌ Failed to generate bindings"
        exit 1
    fi
    
    echo "✅ Bindings generated successfully"
    
    # Step 2: Compile to WebAssembly
    echo "🔨 Compiling to WebAssembly..."
    ../.venv/bin/componentize-py \
        -d ${SDK_DIR}/wit \
        -w wasmcp:mcp/mcp-handler \
        componentize app \
        -p ${SDK_DIR}/src \
        -p . \
        -p bindings \
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