#!/bin/bash
# Sync versions across ftl-components repository

set -euo pipefail

# Get script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$SCRIPT_DIR/.."

# Read versions from versions.toml
VERSIONS_FILE="$REPO_ROOT/versions.toml"

# Extract versions using grep and sed
MCP_HTTP_COMPONENT=$(grep '^mcp-http-component = ' "$VERSIONS_FILE" | sed 's/.*"\(.*\)".*/\1/')
FTL_SDK_RUST=$(grep '^ftl-sdk-rust = ' "$VERSIONS_FILE" | sed 's/.*"\(.*\)".*/\1/')
FTL_SDK_TYPESCRIPT=$(grep '^ftl-sdk-typescript = ' "$VERSIONS_FILE" | sed 's/.*"\(.*\)".*/\1/')
WIT_MCP=$(grep '^mcp = ' "$VERSIONS_FILE" | sed 's/.*"\(.*\)".*/\1/')
MCP_GATEWAY_REF=$(grep '"ghcr.io/bowlofarugula/mcp-http-component" = ' "$VERSIONS_FILE" | sed 's/.*"\(.*\)".*/\1/')

echo "Synchronizing versions across ftl-components..."
echo

# Update Rust template
echo "Updating Rust template..."
sed -i '' "s/ftl-sdk = \"[^\"]*\"/ftl-sdk = \"$FTL_SDK_RUST\"/" \
    "$REPO_ROOT/templates/rust/content/handler/Cargo.toml"

# Update JavaScript template
echo "Updating JavaScript template..."
sed -i '' "s/\"@fastertools\/ftl-sdk\": \"[^\"]*\"/\"@fastertools\/ftl-sdk\": \"^$FTL_SDK_TYPESCRIPT\"/" \
    "$REPO_ROOT/templates/javascript/content/handler/package.json"

# Update TypeScript template
echo "Updating TypeScript template..."
sed -i '' "s/\"@fastertools\/ftl-sdk\": \"[^\"]*\"/\"@fastertools\/ftl-sdk\": \"^$FTL_SDK_TYPESCRIPT\"/" \
    "$REPO_ROOT/templates/typescript/content/handler/package.json"

# Update spin.toml references in all templates
for template in rust javascript typescript; do
    echo "Updating $template spin.toml..."
    spin_toml="$REPO_ROOT/templates/$template/content/spin.toml"
    if [ -f "$spin_toml" ]; then
        sed -i '' "s/bowlofarugula:mcp-gateway\", version = \"[^\"]*\"/bowlofarugula:mcp-gateway\", version = \"$MCP_GATEWAY_REF\"/" "$spin_toml"
    fi
    
    # Update snippet
    snippet="$REPO_ROOT/templates/$template/metadata/snippets/component.txt"
    if [ -f "$snippet" ]; then
        sed -i '' "s/bowlofarugula:mcp-gateway\", version = \"[^\"]*\"/bowlofarugula:mcp-gateway\", version = \"$MCP_GATEWAY_REF\"/" "$snippet"
    fi
done

echo
echo "Version sync complete!"
echo
echo "Current versions:"
echo "  mcp-http-component: $MCP_HTTP_COMPONENT"
echo "  ftl-sdk-rust: $FTL_SDK_RUST"
echo "  ftl-sdk-typescript: $FTL_SDK_TYPESCRIPT"
echo "  WIT package: $WIT_MCP"
echo "  Gateway reference: $MCP_GATEWAY_REF"