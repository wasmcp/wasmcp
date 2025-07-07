#!/bin/bash
# Sync versions across wasmcp repository

set -euo pipefail

# Get script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$SCRIPT_DIR/.."

# Detect OS for sed compatibility
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS - requires backup extension
    sed_inplace() {
        sed -i '' "$@"
    }
else
    # Linux - no backup extension
    sed_inplace() {
        sed -i "$@"
    }
fi

# Read versions from versions.toml
VERSIONS_FILE="$REPO_ROOT/versions.toml"

# Extract versions using grep and sed
WASMCP_SPIN=$(grep '^wasmcp-spin = ' "$VERSIONS_FILE" | sed 's/.*"\(.*\)".*/\1/')
WASMCP_RUST=$(grep '^wasmcp-rust = ' "$VERSIONS_FILE" | sed 's/.*"\(.*\)".*/\1/')
WASMCP_TYPESCRIPT=$(grep '^wasmcp-typescript = ' "$VERSIONS_FILE" | sed 's/.*"\(.*\)".*/\1/')
WIT_MCP=$(grep '^mcp = ' "$VERSIONS_FILE" | sed 's/.*"\(.*\)".*/\1/')
WASMCP_SPIN_REF=$(grep '"ghcr.io/fastertools/wasmcp-spin" = ' "$VERSIONS_FILE" | sed 's/.*"\(.*\)".*/\1/')

echo "Synchronizing versions across wasmcp..."
echo

# Update Rust template
echo "Updating Rust template..."
sed_inplace "s/wasmcp = \"[^\"]*\"/wasmcp = \"$WASMCP_RUST\"/" \
    "$REPO_ROOT/templates/rust/content/handler/Cargo.toml"

# Update JavaScript template
echo "Updating JavaScript template..."
sed_inplace "s/\"wasmcp\": \"[^\"]*\"/\"wasmcp\": \"^$WASMCP_TYPESCRIPT\"/" \
    "$REPO_ROOT/templates/javascript/content/handler/package.json"

# Update TypeScript template
echo "Updating TypeScript template..."
sed_inplace "s/\"wasmcp\": \"[^\"]*\"/\"wasmcp\": \"^$WASMCP_TYPESCRIPT\"/" \
    "$REPO_ROOT/templates/typescript/content/handler/package.json"

# Update spin.toml references in all templates
for template in rust javascript typescript; do
    echo "Updating $template spin.toml..."
    spin_toml="$REPO_ROOT/templates/$template/content/spin.toml"
    if [ -f "$spin_toml" ]; then
        sed_inplace "s/fastertools:wasmcp-spin\", version = \"[^\"]*\"/fastertools:wasmcp-spin\", version = \"$WASMCP_SPIN_REF\"/" "$spin_toml"
    fi
    
    # Update snippet
    snippet="$REPO_ROOT/templates/$template/metadata/snippets/component.txt"
    if [ -f "$snippet" ]; then
        sed_inplace "s/fastertools:wasmcp-spin\", version = \"[^\"]*\"/fastertools:wasmcp-spin\", version = \"$WASMCP_SPIN_REF\"/" "$snippet"
    fi
done

echo
echo "Version sync complete!"
echo
echo "Current versions:"
echo "  wasmcp-spin: $WASMCP_SPIN"
echo "  wasmcp-rust: $WASMCP_RUST"
echo "  wasmcp-typescript: $WASMCP_TYPESCRIPT"
echo "  WIT package: $WIT_MCP"
echo "  Gateway reference: $WASMCP_SPIN_REF"