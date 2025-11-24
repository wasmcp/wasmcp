#!/bin/bash
set -e

echo "==================================================================="
echo "Todo List Auth Example - Test Environment Setup"
echo "==================================================================="
echo ""

# Use local development build of wasmcp CLI
WASMCP_CLI="/Users/coreyryan/data/mashh/wasmcp_2/cli/target/aarch64-apple-darwin/release/wasmcp"

# Ensure wasmcp CLI is available
if [ ! -f "$WASMCP_CLI" ]; then
    echo "❌ Error: wasmcp CLI not found at $WASMCP_CLI"
    echo "   Please build the wasmcp CLI first:"
    echo "   cargo dt build --only cli"
    exit 1
fi

echo "Step 1: Generate RSA keypair for JWT testing"
echo "-------------------------------------------------------------------"
$WASMCP_CLI jwt generate-keypair --force
echo ""

echo "Step 2: Create test tokens for different user personas"
echo "-------------------------------------------------------------------"
echo ""

# Admin user - full access
echo "Creating ADMIN token (full access)..."
$WASMCP_CLI jwt mint \
  --subject "admin@example.com" \
  --audience "http://localhost:3000" \
  --scope "mcp:read mcp:write" \
  --claim role=admin \
  --save-as admin \
  > /dev/null

echo "✓ admin token created"
echo ""

# Read-only user - can only read
echo "Creating READONLY token (mcp:read only)..."
$WASMCP_CLI jwt mint \
  --subject "readonly@example.com" \
  --audience "http://localhost:3000" \
  --scope "mcp:read" \
  --claim role=viewer \
  --save-as readonly \
  > /dev/null

echo "✓ readonly token created"
echo ""

# Analyst user - read/write but limited tools
echo "Creating ANALYST token (limited tools)..."
$WASMCP_CLI jwt mint \
  --subject "analyst@example.com" \
  --audience "http://localhost:3000" \
  --scope "mcp:read mcp:write" \
  --claim role=analyst \
  --claim allowed_tools="add_item,list_items" \
  --save-as analyst \
  > /dev/null

echo "✓ analyst token created"
echo ""

echo "Step 3: Verify tokens"
echo "-------------------------------------------------------------------"
$WASMCP_CLI jwt list-tokens
echo ""

echo "==================================================================="
echo "Setup Complete! ✅"
echo "==================================================================="
echo ""
echo "Test tokens created:"
echo "  • admin    - Full access (mcp:read, mcp:write, role=admin)"
echo "  • readonly - Read only (mcp:read, role=viewer)"
echo "  • analyst  - Limited tools (mcp:read, mcp:write, allowed_tools=add_item,list_items)"
echo ""
echo "To use a token:"
echo "  export JWT_TOKEN=\$($WASMCP_CLI jwt load-token admin)"
echo ""
echo "To decode and inspect a token:"
echo "  $WASMCP_CLI jwt decode-token admin"
echo ""
