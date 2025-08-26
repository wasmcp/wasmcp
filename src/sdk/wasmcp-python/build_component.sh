#!/bin/bash
set -e

echo "Building wasmcp Python component..."

# Set up paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VENV_PATH="/Users/coreyryan/data/mashh/wasmcp/.agent/venv"
SITE_PACKAGES="$VENV_PATH/lib/python3.13/site-packages"
WIT_DIR="$SCRIPT_DIR/wit"
SRC_DIR="$SCRIPT_DIR/src"
EXAMPLES_DIR="$SCRIPT_DIR/examples"
OUTPUT_WASM="$SCRIPT_DIR/test.wasm"

echo "Paths:"
echo "  Script dir: $SCRIPT_DIR"
echo "  WIT dir: $WIT_DIR" 
echo "  Source dir: $SRC_DIR"
echo "  Examples dir: $EXAMPLES_DIR"
echo "  Output: $OUTPUT_WASM"

# Change to examples directory
cd "$EXAMPLES_DIR"

echo "Building with componentize-py..."
"$VENV_PATH/bin/componentize-py" \
    -d "$WIT_DIR" \
    -w mcp-handler \
    componentize test_component \
    -p "$SITE_PACKAGES" \
    -p "$SRC_DIR" \
    -p "$EXAMPLES_DIR" \
    -o "$OUTPUT_WASM"

echo "Build completed: $OUTPUT_WASM"