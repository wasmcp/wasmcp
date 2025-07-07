#!/bin/bash
# Bump version for a specific component

set -euo pipefail

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

if [ $# -lt 2 ]; then
    echo "Usage: $0 <component> <new-version>"
    echo ""
    echo "Components:"
    echo "  wasmcp-spin         - Spin gateway component"
    echo "  wasmcp-rust         - Rust SDK"
    echo "  wasmcp-typescript   - TypeScript SDK"
    echo "  mcp                 - MCP WIT interface (breaking changes only)"
    echo "  wasmcp-spin-wit     - Spin WIT interface"
    echo ""
    echo "Examples:"
    echo "  $0 wasmcp-rust 0.3.0"
    echo "  $0 mcp 0.2.0"
    exit 1
fi

COMPONENT=$1
NEW_VERSION=$2
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
VERSIONS_FILE="$SCRIPT_DIR/../versions.toml"

# Update versions.toml
case $COMPONENT in
    wasmcp-spin)
        sed_inplace "s/wasmcp-spin = \"[^\"]*\"/wasmcp-spin = \"$NEW_VERSION\"/" "$VERSIONS_FILE"
        sed_inplace "s/ghcr.io\/fastertools\/wasmcp-spin\" = \"[^\"]*\"/ghcr.io\/fastertools\/wasmcp-spin\" = \"$NEW_VERSION\"/" "$VERSIONS_FILE"
        
        # Also update the actual Cargo.toml (only the package version line)
        sed_inplace "/^\[package\]/,/^\[/ s/^version = \"[^\"]*\"/version = \"$NEW_VERSION\"/" "$SCRIPT_DIR/../src/components/wasmcp-spin/Cargo.toml"
        
        # Update package.metadata.component version if wasmcp-spin-wit version changed
        WASMCP_SPIN_WIT_VERSION=$(grep '^wasmcp-spin-wit = ' "$VERSIONS_FILE" | sed 's/.*"\(.*\)".*/\1/')
        sed_inplace "s/package = \"wasmcp:spin@[^\"]*\"/package = \"wasmcp:spin@$WASMCP_SPIN_WIT_VERSION\"/" "$SCRIPT_DIR/../src/components/wasmcp-spin/Cargo.toml"
        ;;
    wasmcp-rust)
        sed_inplace "s/wasmcp-rust = \"[^\"]*\"/wasmcp-rust = \"$NEW_VERSION\"/" "$VERSIONS_FILE"
        sed_inplace "s/crates.io\/wasmcp\" = \"[^\"]*\"/crates.io\/wasmcp\" = \"$NEW_VERSION\"/" "$VERSIONS_FILE"
        
        # Also update the actual Cargo.toml (only the package version line)
        sed_inplace "/^\[package\]/,/^\[/ s/^version = \"[^\"]*\"/version = \"$NEW_VERSION\"/" "$SCRIPT_DIR/../src/sdk/wasmcp-rust/Cargo.toml"
        ;;
    wasmcp-typescript)
        sed_inplace "s/wasmcp-typescript = \"[^\"]*\"/wasmcp-typescript = \"$NEW_VERSION\"/" "$VERSIONS_FILE"
        sed_inplace "s/npm\/wasmcp\" = \"[^\"]*\"/npm\/wasmcp\" = \"$NEW_VERSION\"/" "$VERSIONS_FILE"
        
        # Also update the actual package.json
        cd "$SCRIPT_DIR/../src/sdk/wasmcp-typescript"
        npm version "$NEW_VERSION" --no-git-tag-version
        ;;
    mcp)
        sed_inplace "s/mcp = \"[^\"]*\"/mcp = \"$NEW_VERSION\"/" "$VERSIONS_FILE"
        
        # Also update WIT files
        sed_inplace "s/package wasmcp:mcp@[^;]*/package wasmcp:mcp@$NEW_VERSION/" "$SCRIPT_DIR/../wit/mcp.wit"
        ;;
    wasmcp-spin-wit)
        sed_inplace "s/wasmcp-spin-wit = \"[^\"]*\"/wasmcp-spin-wit = \"$NEW_VERSION\"/" "$VERSIONS_FILE"
        
        # Also update WIT files and Cargo.toml
        sed_inplace "s/package wasmcp:spin@[^;]*/package wasmcp:spin@$NEW_VERSION/" "$SCRIPT_DIR/../src/components/wasmcp-spin/wit/world.wit"
        sed_inplace "s/package = \"wasmcp:spin@[^\"]*\"/package = \"wasmcp:spin@$NEW_VERSION\"/" "$SCRIPT_DIR/../src/components/wasmcp-spin/Cargo.toml"
        ;;
    *)
        echo "Unknown component: $COMPONENT"
        exit 1
        ;;
esac

echo "Updated $COMPONENT to version $NEW_VERSION"
echo ""
echo "Now running version sync..."
"$SCRIPT_DIR/sync-versions.sh"

echo ""
echo "Done! Don't forget to:"
echo "  1. Commit the changes"
echo "  2. Create a git tag: git tag ${COMPONENT}-v${NEW_VERSION}"
echo "  3. Push the tag: git push origin ${COMPONENT}-v${NEW_VERSION}"