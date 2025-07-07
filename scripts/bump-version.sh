#!/bin/bash
# Bump version for a specific component

set -euo pipefail

# Detect OS for sed compatibility
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    SED_INPLACE="sed -i ''"
else
    # Linux
    SED_INPLACE="sed -i"
fi

if [ $# -lt 2 ]; then
    echo "Usage: $0 <component> <new-version>"
    echo ""
    echo "Components:"
    echo "  mcp-http-component"
    echo "  ftl-sdk-rust"
    echo "  ftl-sdk-typescript"
    echo "  wit"
    echo ""
    echo "Example: $0 ftl-sdk-rust 0.3.0"
    exit 1
fi

COMPONENT=$1
NEW_VERSION=$2
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
VERSIONS_FILE="$SCRIPT_DIR/../versions.toml"

# Update versions.toml
case $COMPONENT in
    mcp-http-component)
        $SED_INPLACE "s/mcp-http-component = \"[^\"]*\"/mcp-http-component = \"$NEW_VERSION\"/" "$VERSIONS_FILE"
        $SED_INPLACE "s/ghcr.io\/bowlofarugula\/mcp-http-component\" = \"[^\"]*\"/ghcr.io\/bowlofarugula\/mcp-http-component\" = \"$NEW_VERSION\"/" "$VERSIONS_FILE"
        
        # Also update the actual Cargo.toml (only the package version line)
        $SED_INPLACE "/^\[package\]/,/^\[/ s/^version = \"[^\"]*\"/version = \"$NEW_VERSION\"/" "$SCRIPT_DIR/../src/mcp-http-component/Cargo.toml"
        ;;
    ftl-sdk-rust)
        $SED_INPLACE "s/ftl-sdk-rust = \"[^\"]*\"/ftl-sdk-rust = \"$NEW_VERSION\"/" "$VERSIONS_FILE"
        $SED_INPLACE "s/crates.io\/ftl-sdk\" = \"[^\"]*\"/crates.io\/ftl-sdk\" = \"$NEW_VERSION\"/" "$VERSIONS_FILE"
        
        # Also update the actual Cargo.toml (only the package version line)
        $SED_INPLACE "/^\[package\]/,/^\[/ s/^version = \"[^\"]*\"/version = \"$NEW_VERSION\"/" "$SCRIPT_DIR/../src/ftl-sdk-rust/Cargo.toml"
        ;;
    ftl-sdk-typescript)
        $SED_INPLACE "s/ftl-sdk-typescript = \"[^\"]*\"/ftl-sdk-typescript = \"$NEW_VERSION\"/" "$VERSIONS_FILE"
        $SED_INPLACE "s/npm\/@fastertools\/ftl-sdk\" = \"[^\"]*\"/npm\/@fastertools\/ftl-sdk\" = \"$NEW_VERSION\"/" "$VERSIONS_FILE"
        
        # Also update the actual package.json
        cd "$SCRIPT_DIR/../src/ftl-sdk-typescript"
        npm version "$NEW_VERSION" --no-git-tag-version
        ;;
    wit)
        $SED_INPLACE "s/mcp = \"[^\"]*\"/mcp = \"$NEW_VERSION\"/" "$VERSIONS_FILE"
        
        # Also update WIT files
        $SED_INPLACE "s/package component:mcp@[^;]*/package component:mcp@$NEW_VERSION/" "$SCRIPT_DIR/../wit/mcp.wit"
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