#!/bin/bash

# Build the combined MCP + Spin Python component to WebAssembly
echo "Building combined MCP + Spin Python component..."

# Default to app world, but allow override
WORLD="${WORLD:-app}"
echo "🌍 Building for world: $WORLD"

# Check if we're doing a local build (with editable SDK install)
if [ "$LOCAL_BUILD" = "true" ]; then
    echo "🔧 LOCAL BUILD MODE - Using relative paths for editable-installed SDK"
    echo "   Note: In production, wasmcp-python would be pip-installed and WIT files would be bundled"
    
    # For local development with -e installed SDK, we need explicit paths
    SDK_DIR="/Users/coreyryan/data/mashh/wasmcp/src/sdk/wasmcp-python"
    
    echo "✅ Using wasmcp SDK from: ${SDK_DIR}"
    
    # Step 1: Verify dependencies exist
    if [ ! -f "/Users/coreyryan/data/mashh/wasmcp/src/sdk/wasmcp-python/src/wasmcp/wit/world.wit" ]; then
        echo "❌ Error: wasmcp SDK WIT files not found at ${SDK_DIR}/src/wasmcp/wit/"
        echo "   Make sure the SDK is properly set up"
        exit 1
    fi
    
    echo "📦 Dependencies verified"
    
    # Step 2: Build with componentize-py
    echo "🔨 Compiling to WebAssembly with componentize-py..."
    
    # Build the combined component
    echo "🔨 Building with module-world mapping for spin_sdk (no file copying)..."
    componentize-py \
        -d ./wit \
        -w app \
        componentize src.app \
        -o app.wasm

else
    # Production mode - expects both wasmcp-python and spin-sdk to be properly installed
    echo "🏭 PRODUCTION BUILD MODE"
    echo "   Note: This assumes both wasmcp-python and spin-sdk are pip-installed with bundled WIT definitions"
    
    # Step 1: Verify virtual environment and dependencies
    if [ ! -d "./.venv" ]; then
        echo "❌ Error: Virtual environment not found at ./.venv"
        echo "   Create it with: python -m venv .venv && source .venv/bin/activate && pip install wasmcp spin-sdk"
        exit 1
    fi
    
    echo "📦 Using production virtual environment"
    
    # Step 2: Build with installed packages
    ./.venv/bin/componentize-py \
        -d . \
        -w ${WORLD} \
        componentize src.app \
        -p ./.venv/lib/python3.13/site-packages \
        -p ./src \
        -o app.wasm
fi

# Check build result
if [ $? -eq 0 ]; then
    echo "✅ Successfully built app.wasm"
    ls -lh app.wasm
    echo ""
    echo "🎉 Component ready! This component exports both:"
    echo "   • MCP handler interface (tools, resources, prompts)"  
    echo "   • Spin HTTP interface (REST API endpoints)"
    echo ""
    echo "📝 Test it with:"
    echo "   • MCP: Use as dependency in spin.toml"
    echo "   • HTTP: Deploy with 'spin deploy' and test endpoints"
else
    echo "❌ Build failed"
    if [ "$LOCAL_BUILD" != "true" ]; then
        echo "   Hint: If using editable-installed SDK, set LOCAL_BUILD=true"
    fi
    exit 1
fi