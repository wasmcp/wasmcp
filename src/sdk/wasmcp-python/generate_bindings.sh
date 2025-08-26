#!/bin/bash

# Generate Python bindings from WIT files for the wasmcp SDK
# This script generates the bindings that componentize-py needs to work with our WIT world

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK_DIR="$SCRIPT_DIR"
WIT_DIR="$SDK_DIR/wit"
OUTPUT_DIR="$SDK_DIR/src/wasmcp/wit"

echo "🔧 Generating Python bindings from WIT files..."
echo "   WIT directory: $WIT_DIR"
echo "   Output directory: $OUTPUT_DIR"

# Ensure output directory exists
mkdir -p "$OUTPUT_DIR"

# Check if we're in a virtual environment with componentize-py
if ! command -v componentize-py &> /dev/null; then
    echo "❌ componentize-py not found. Looking for it in examples venv..."
    VENV_COMPONENTIZE="/Users/coreyryan/data/mashh/wasmcp/examples/python-echo/.venv/bin/componentize-py"
    if [ -f "$VENV_COMPONENTIZE" ]; then
        echo "✅ Using componentize-py from examples venv"
        COMPONENTIZE_PY="$VENV_COMPONENTIZE"
    else
        echo "❌ componentize-py not found. Please install it:"
        echo "   pip install componentize-py"
        exit 1
    fi
else
    COMPONENTIZE_PY="componentize-py"
fi

# Clean old bindings
echo "🧹 Cleaning old bindings..."
rm -rf "$OUTPUT_DIR/bindings"

# Generate bindings from our WIT world
# componentize-py reads WIT config from pyproject.toml
echo "📝 Generating bindings for mcp-handler world..."
cd "$SDK_DIR" && "$COMPONENTIZE_PY" bindings "$OUTPUT_DIR/bindings"

if [ $? -eq 0 ]; then
    echo "✅ Bindings generated successfully in $OUTPUT_DIR"
    
    # List what was generated
    echo "📦 Generated files:"
    find "$OUTPUT_DIR" -type f -name "*.py" | sort
else
    echo "❌ Failed to generate bindings"
    exit 1
fi