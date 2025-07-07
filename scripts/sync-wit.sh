#!/bin/bash
# Sync WIT files to mcp-http-component

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$SCRIPT_DIR/.."

echo "Syncing WIT files..."

# Copy the main WIT file
cp "$REPO_ROOT/wit/mcp.wit" "$REPO_ROOT/src/mcp-http-component/wit/handler.wit"

# Create a temporary file
temp_file=$(mktemp)

# Extract only the interface definition (everything between "interface handler {" and its closing "}")
awk '
    /^interface handler \{/ { in_interface = 1 }
    in_interface { print }
    /^}/ && in_interface { in_interface = 0; exit }
' "$REPO_ROOT/src/mcp-http-component/wit/handler.wit" > "$temp_file"

# Replace the original file
mv "$temp_file" "$REPO_ROOT/src/mcp-http-component/wit/handler.wit"

echo "✅ WIT files synced"