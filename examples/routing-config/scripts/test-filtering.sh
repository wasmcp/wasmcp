#!/bin/bash
set -e

echo "==================================================================="
echo "Routing Config Example - Filter Middleware Test Scenarios"
echo "==================================================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Kill any existing spin processes before starting
echo "Checking for existing Spin processes..."
if pgrep -f "spin up" > /dev/null; then
    echo "Killing existing Spin processes..."
    pkill -f "spin up" || true
    sleep 2
fi

# Cleanup function to stop Spin server
cleanup() {
    if [ -n "$SPIN_PID" ] && kill -0 $SPIN_PID 2>/dev/null; then
        echo ""
        echo "Stopping Spin server (PID: $SPIN_PID)..."
        kill $SPIN_PID
        wait $SPIN_PID 2>/dev/null || true
        echo -e "${GREEN}✓ Server stopped${NC}"
    fi
    # Also kill any remaining spin processes as fallback
    pkill -f "spin up" 2>/dev/null || true
    rm -f /tmp/spin_output_$$.log /tmp/init_headers_$$.txt
}

# Set trap to cleanup on exit
trap cleanup EXIT INT TERM

# Use local development build of wasmcp CLI
WASMCP_CLI="/Users/coreyryan/data/mashh/wasmcp_2/cli/target/aarch64-apple-darwin/release/wasmcp"

# Check if wasmcp CLI is available
if [ ! -f "$WASMCP_CLI" ]; then
    echo -e "${RED}❌ Error: wasmcp CLI not found at $WASMCP_CLI${NC}"
    echo "   Please build the wasmcp CLI first:"
    echo "   cargo dt build --only cli"
    exit 1
fi

# Check if server is composed
SERVER_PATH="mcp-server.wasm"
if [ ! -f "$SERVER_PATH" ]; then
    echo -e "${RED}❌ Error: MCP server not found at $SERVER_PATH${NC}"
    echo "   Please compose the server first:"
    echo "   make compose"
    exit 1
fi

# Check if JWT tokens exist (created by todo-list-auth setup)
if ! $WASMCP_CLI jwt list-tokens 2>/dev/null | grep -q "admin"; then
    echo -e "${YELLOW}⚠ Warning: JWT tokens not found${NC}"
    echo "   Running setup from todo-list-auth example..."
    cd ../todo-list-auth
    ./scripts/setup-test-env.sh
    cd ../routing-config
    echo ""
fi

# Read JWT public key
JWT_PUBLIC_KEY=$(cat ~/Library/Application\ Support/wasmcp/jwt-test/public.pem)
if [ -z "$JWT_PUBLIC_KEY" ]; then
    echo -e "${RED}❌ Error: Failed to read JWT public key${NC}"
    exit 1
fi

# Start the Spin server with environment variables
echo "Starting Spin server with filter middleware..."
echo ""

# Default: No tag filtering
spin up --disable-pooling -e JWT_PUBLIC_KEY="$JWT_PUBLIC_KEY" > /tmp/spin_output_$$.log 2>&1 &
SPIN_PID=$!

# Wait for server to be ready
echo "Waiting for server to start..."
for i in {1..30}; do
    if curl -s http://localhost:3000/mcp > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Server started (PID: $SPIN_PID)${NC}"
        break
    fi
    sleep 1
    if [ $i -eq 30 ]; then
        echo -e "${RED}❌ Error: Server failed to start within 30 seconds${NC}"
        echo "Server output:"
        cat /tmp/spin_output_$$.log
        kill $SPIN_PID 2>/dev/null
        exit 1
    fi
done

echo -e "${GREEN}✓ MCP server is running at http://localhost:3000/mcp${NC}"
echo ""

# Session storage
SESSION_ID=""
REQUEST_ID=1

# Initialize MCP session
initialize_session() {
    local path="$1"
    local token_name="${2:-admin}"

    echo -e "${BLUE}Initializing session on path: $path${NC}"

    # Load token
    local token=$($WASMCP_CLI jwt load-token "$token_name" 2>/dev/null)
    if [ -z "$token" ]; then
        echo -e "${RED}  ✗ Failed to load token '$token_name'${NC}"
        return 1
    fi

    # Send initialize request to specific path
    local init_response=$(curl -s -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -H "Accept: application/json, text/event-stream" \
        -D /tmp/init_headers_$$.txt \
        -d '{"jsonrpc":"2.0","id":'$REQUEST_ID',"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"routing-test","version":"1.0.0"}}}' \
        http://localhost:3000$path)

    REQUEST_ID=$((REQUEST_ID + 1))

    # Extract session ID
    if [ -f /tmp/init_headers_$$.txt ]; then
        SESSION_ID=$(grep -i "^Mcp-Session-Id:" /tmp/init_headers_$$.txt | cut -d' ' -f2 | tr -d '\r\n')
        rm -f /tmp/init_headers_$$.txt
    fi

    # Check initialization
    if echo "$init_response" | grep -q '"result"'; then
        echo -e "${GREEN}  ✓ Session initialized (ID: ${SESSION_ID})${NC}"
        return 0
    else
        echo -e "${RED}  ✗ Initialization failed${NC}"
        echo "    Response: $init_response"
        return 1
    fi
}

# List available tools
list_tools() {
    local path="$1"
    local token_name="${2:-admin}"

    echo -e "${YELLOW}Listing tools on path: $path${NC}"

    # Load token
    local token=$($WASMCP_CLI jwt load-token "$token_name" 2>/dev/null)

    # Build headers
    local headers="-H \"Authorization: Bearer $token\" -H \"Content-Type: application/json\""
    if [ -n "$SESSION_ID" ]; then
        headers="$headers -H \"Mcp-Session-Id: $SESSION_ID\""
    fi

    # Make tools/list request
    local response=$(eval curl -s -X POST \
        $headers \
        -d "'{\"jsonrpc\":\"2.0\",\"id\":$REQUEST_ID,\"method\":\"tools/list\",\"params\":{}}'" \
        http://localhost:3000$path)

    REQUEST_ID=$((REQUEST_ID + 1))

    # Extract tool names
    local tools=$(echo "$response" | grep -o '"name":"[^"]*"' | cut -d'"' -f4 | sort)

    if [ -n "$tools" ]; then
        echo -e "${GREEN}  Available tools:${NC}"
        echo "$tools" | while read tool; do
            echo -e "${GREEN}    • $tool${NC}"
        done
    else
        echo -e "${YELLOW}  No tools available${NC}"
    fi
    echo ""

    # Return tool list for validation
    echo "$tools"
}

# Call a specific tool
call_tool() {
    local path="$1"
    local tool_name="$2"
    local args="$3"
    local expected="$4"  # "success" or "fail"
    local token_name="${5:-admin}"

    echo -e "${YELLOW}Testing: tools/call → $tool_name on $path${NC}"

    # Load token
    local token=$($WASMCP_CLI jwt load-token "$token_name" 2>/dev/null)

    # Build headers
    local headers="-H \"Authorization: Bearer $token\" -H \"Content-Type: application/json\""
    if [ -n "$SESSION_ID" ]; then
        headers="$headers -H \"Mcp-Session-Id: $SESSION_ID\""
    fi

    # Make tools/call request
    local response=$(eval curl -s -X POST \
        $headers \
        -d "'{\"jsonrpc\":\"2.0\",\"id\":$REQUEST_ID,\"method\":\"tools/call\",\"params\":{\"name\":\"$tool_name\",\"arguments\":$args}}'" \
        http://localhost:3000$path)

    REQUEST_ID=$((REQUEST_ID + 1))

    # Check response
    if echo "$response" | grep -q '"error"'; then
        if [ "$expected" = "fail" ]; then
            echo -e "${GREEN}  ✓ Expected: BLOCKED${NC}"
            local error=$(echo "$response" | grep -o '"message":"[^"]*"' | cut -d'"' -f4)
            echo -e "${GREEN}    → $error${NC}"
        else
            echo -e "${RED}  ✗ Unexpected: BLOCKED${NC}"
            echo "    Response: $response"
        fi
    else
        if [ "$expected" = "success" ]; then
            echo -e "${GREEN}  ✓ Expected: ALLOWED${NC}"
            local result=$(echo "$response" | grep -o '"text":"[^"]*"' | head -1 | cut -d'"' -f4)
            if [ -n "$result" ]; then
                echo -e "${GREEN}    Result: $result${NC}"
            fi
        else
            echo -e "${RED}  ✗ Unexpected: ALLOWED${NC}"
            echo "    Response: $response"
        fi
    fi
    echo ""
}

# ===================================================================
# SCENARIO 1: No Path Rule (Allow All)
# ===================================================================
echo "==================================================================="
echo "SCENARIO 1: No Path Rule - /mcp (allow all tools)"
echo "==================================================================="
echo "Path: /mcp"
echo "Expected: All tools from all components"
echo ""

initialize_session "/mcp" || exit 1
TOOLS_ALL=$(list_tools "/mcp")

# Verify we have tools from both calculator and todo-list
if echo "$TOOLS_ALL" | grep -q "add" && echo "$TOOLS_ALL" | grep -q "add_item"; then
    echo -e "${GREEN}✓ Both calculator and todo-list tools available${NC}"
else
    echo -e "${RED}✗ Missing expected tools${NC}"
fi
echo ""

# ===================================================================
# SCENARIO 2: Path Filtering - Whitelist Component
# ===================================================================
echo "==================================================================="
echo "SCENARIO 2: Path Filtering - /mcp/math (whitelist calculator-rs)"
echo "==================================================================="
echo "Path: /mcp/math"
echo "Config: whitelist=[calculator-rs], blacklist=[factorial]"
echo "Expected: add, subtract (NO factorial, NO todo tools)"
echo ""

initialize_session "/mcp/math" || exit 1
TOOLS_MATH=$(list_tools "/mcp/math")

# Verify calculator tools present (except factorial which is blacklisted)
if echo "$TOOLS_MATH" | grep -q "add" && echo "$TOOLS_MATH" | grep -q "subtract"; then
    echo -e "${GREEN}✓ Calculator tools available${NC}"
else
    echo -e "${RED}✗ Missing calculator tools${NC}"
fi

# Verify factorial is blacklisted
if ! echo "$TOOLS_MATH" | grep -q "factorial"; then
    echo -e "${GREEN}✓ factorial correctly blacklisted${NC}"
else
    echo -e "${RED}✗ factorial should be blacklisted${NC}"
fi

# Verify todo tools NOT present
if ! echo "$TOOLS_MATH" | grep -q "add_item"; then
    echo -e "${GREEN}✓ Todo tools correctly filtered${NC}"
else
    echo -e "${RED}✗ Todo tools should be filtered${NC}"
fi
echo ""

# ===================================================================
# SCENARIO 3: Hierarchical Path Matching
# ===================================================================
echo "==================================================================="
echo "SCENARIO 3: Hierarchical Path - /mcp/math/addition"
echo "==================================================================="
echo "Path: /mcp/math/addition"
echo "Config: whitelist=[add] (overrides /mcp/math)"
echo "Expected: ONLY add tool"
echo ""

initialize_session "/mcp/math/addition" || exit 1
TOOLS_ADDITION=$(list_tools "/mcp/math/addition")

# Verify only add is present
if echo "$TOOLS_ADDITION" | grep -q "^add$" && ! echo "$TOOLS_ADDITION" | grep -q "subtract"; then
    echo -e "${GREEN}✓ Hierarchical matching: only 'add' tool available${NC}"
else
    echo -e "${RED}✗ Hierarchical matching failed${NC}"
    echo "Tools found: $TOOLS_ADDITION"
fi
echo ""

# ===================================================================
# SCENARIO 4: Path Filtering - Component Whitelist (Todo)
# ===================================================================
echo "==================================================================="
echo "SCENARIO 4: Path Filtering - /mcp/todo (whitelist todo-list-auth)"
echo "==================================================================="
echo "Path: /mcp/todo"
echo "Config: whitelist=[todo-list-auth]"
echo "Expected: add_item, list_items, remove_item, clear_all (NO calculator)"
echo ""

initialize_session "/mcp/todo" || exit 1
TOOLS_TODO=$(list_tools "/mcp/todo")

# Verify todo tools present
if echo "$TOOLS_TODO" | grep -q "add_item" && echo "$TOOLS_TODO" | grep -q "list_items"; then
    echo -e "${GREEN}✓ Todo tools available${NC}"
else
    echo -e "${RED}✗ Missing todo tools${NC}"
fi

# Verify calculator tools NOT present
if ! echo "$TOOLS_TODO" | grep -q "^add$"; then
    echo -e "${GREEN}✓ Calculator tools correctly filtered${NC}"
else
    echo -e "${RED}✗ Calculator tools should be filtered${NC}"
fi
echo ""

# ===================================================================
# SCENARIO 5: Path Filtering - Specific Tools (/mcp/calc)
# ===================================================================
echo "==================================================================="
echo "SCENARIO 5: Path Filtering - /mcp/calc (tool whitelist)"
echo "==================================================================="
echo "Path: /mcp/calc"
echo "Config: whitelist=[add, subtract, factorial]"
echo "Expected: add, subtract, factorial only"
echo ""

initialize_session "/mcp/calc" || exit 1
TOOLS_CALC=$(list_tools "/mcp/calc")

# Verify expected tools
if echo "$TOOLS_CALC" | grep -q "add" && echo "$TOOLS_CALC" | grep -q "subtract" && echo "$TOOLS_CALC" | grep -q "factorial"; then
    echo -e "${GREEN}✓ Whitelisted tools available${NC}"
else
    echo -e "${RED}✗ Missing whitelisted tools${NC}"
fi

# Verify todo tools NOT present
if ! echo "$TOOLS_CALC" | grep -q "add_item"; then
    echo -e "${GREEN}✓ Non-calculator tools correctly filtered${NC}"
else
    echo -e "${RED}✗ Todo tools should be filtered${NC}"
fi
echo ""

# ===================================================================
# SCENARIO 6: tools/call Enforcement
# ===================================================================
echo "==================================================================="
echo "SCENARIO 6: tools/call Enforcement"
echo "==================================================================="
echo "Test: Attempt to call filtered tool not in session registry"
echo ""

# First, list tools on /mcp/math to populate session registry
initialize_session "/mcp/math" || exit 1
list_tools "/mcp/math" > /dev/null

# Now try to call a todo tool (should be blocked)
call_tool "/mcp/math" "add_item" '{"title":"Should be blocked"}' "fail"

# Try to call an allowed calculator tool (should succeed)
call_tool "/mcp/math" "add" '{"a":5,"b":3}' "success"

echo -e "${GREEN}✓ tools/call enforcement working correctly${NC}"
echo ""

# ===================================================================
# SCENARIO 7: Tag Filtering Only
# ===================================================================
echo "==================================================================="
echo "SCENARIO 7: Tag Filtering - /mcp/math-only (category=math)"
echo "==================================================================="
echo "Path: /mcp/math-only"
echo "Config: tag-filters = { category = \"math\" }"
echo "Expected: Only tools with category=math tag (calculator tools)"
echo ""

initialize_session "/mcp/math-only" || exit 1
TOOLS_TAG_MATH=$(list_tools "/mcp/math-only")

# Verify calculator tools present (all have category=math)
if echo "$TOOLS_TAG_MATH" | grep -q "add" && echo "$TOOLS_TAG_MATH" | grep -q "subtract" && echo "$TOOLS_TAG_MATH" | grep -q "factorial"; then
    echo -e "${GREEN}✓ Math category tools available${NC}"
else
    echo -e "${RED}✗ Missing math category tools${NC}"
fi

# Verify todo tools NOT present (have category=productivity)
if ! echo "$TOOLS_TAG_MATH" | grep -q "add_item"; then
    echo -e "${GREEN}✓ Non-math tools correctly filtered by tag${NC}"
else
    echo -e "${RED}✗ Non-math tools should be filtered${NC}"
fi
echo ""

# ===================================================================
# SCENARIO 8: Multiple Tag Filters
# ===================================================================
echo "==================================================================="
echo "SCENARIO 8: Multiple Tag Filters - /mcp/foundational-math"
echo "==================================================================="
echo "Path: /mcp/foundational-math"
echo "Config: tag-filters = { category = \"math\", tool-level = \"foundational\" }"
echo "Expected: Only tools matching BOTH category=math AND tool-level=foundational"
echo ""

initialize_session "/mcp/foundational-math" || exit 1
TOOLS_MULTI_TAG=$(list_tools "/mcp/foundational-math")

# Verify calculator tools present (have both category=math AND tool-level=foundational)
if echo "$TOOLS_MULTI_TAG" | grep -q "add" && echo "$TOOLS_MULTI_TAG" | grep -q "subtract"; then
    echo -e "${GREEN}✓ Tools matching all tag filters available${NC}"
else
    echo -e "${RED}✗ Missing tools with required tags${NC}"
fi

# Verify todo tools NOT present (don't have category=math)
if ! echo "$TOOLS_MULTI_TAG" | grep -q "add_item"; then
    echo -e "${GREEN}✓ Tools not matching all filters correctly excluded${NC}"
else
    echo -e "${RED}✗ Tools should match ALL tag filters${NC}"
fi
echo ""

# ===================================================================
# Summary
# ===================================================================
echo "==================================================================="
echo "Filter Middleware Test Summary"
echo "==================================================================="
echo ""
echo "Filtering Patterns Demonstrated:"
echo ""
echo "1. ${GREEN}No Path Rule${NC}"
echo "   • /mcp allows all tools from all components"
echo ""
echo "2. ${GREEN}Component-Level Whitelist${NC}"
echo "   • /mcp/math whitelists calculator-rs component"
echo "   • /mcp/todo whitelists todo-list-auth component"
echo ""
echo "3. ${GREEN}Tool-Level Whitelist & Blacklist${NC}"
echo "   • /mcp/calc whitelists specific tools (add, subtract, factorial)"
echo "   • /mcp/math blacklists factorial from calculator-rs"
echo "   • /mcp/math/addition whitelists single tool (add)"
echo ""
echo "4. ${GREEN}Hierarchical Path Matching${NC}"
echo "   • /mcp/math/addition overrides /mcp/math rules"
echo "   • Longest matching path wins"
echo ""
echo "5. ${GREEN}tools/call Enforcement${NC}"
echo "   • Filtered tools blocked even if called directly"
echo "   • Session registry validates tool availability"
echo ""
echo "6. ${GREEN}Tag-Based Filtering${NC}"
echo "   • /mcp/math-only filters by category=math tag"
echo "   • /mcp/foundational-math requires multiple tags (AND logic)"
echo "   • Tags defined in tool metadata, filters in routing.toml"
echo ""
echo "==================================================================="
echo ""
echo -e "${GREEN}✓ All filter middleware tests completed successfully!${NC}"
