#!/bin/bash

# This script should be run from the SDK directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Generate Python bindings from WIT files
echo "Generating Python bindings from WIT files..."
echo "Current directory: $(pwd)"

# Update pyproject.toml with componentize-py config
if ! grep -q "\[tool.componentize-py\]" pyproject.toml; then
    echo "" >> pyproject.toml
    echo "[tool.componentize-py]" >> pyproject.toml
    echo "wit-path = \"wit\"" >> pyproject.toml  
    echo "world = \"mcp-handler\"" >> pyproject.toml
fi

# Clean old bindings
rm -rf src/wasmcp/wit/bindings

# Use componentize-py to generate bindings
../../../examples/python-echo/.venv/bin/componentize-py bindings src/wasmcp/wit/bindings

if [ $? -eq 0 ]; then
    echo "✅ Successfully generated Python bindings"
    echo "Generated files:"
    ls -la src/wasmcp/wit/bindings/
else
    echo "❌ Failed to generate bindings"
    exit 1
fi