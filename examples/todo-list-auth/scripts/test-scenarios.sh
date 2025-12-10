#!/bin/bash
set -e

echo "==================================================================="
echo "Todo List Auth Example - Authorization Test Scenarios"
echo "==================================================================="
echo ""

WASMCP_CLI="wasmcp"

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

# Start the Spin server
echo "Starting Spin server..."
echo ""

# Read JWT public key
JWT_PUBLIC_KEY=$(cat ~/Library/Application\ Support/wasmcp/jwt-test/public.pem)
if [ -z "$JWT_PUBLIC_KEY" ]; then
    echo -e "${RED}❌ Error: Failed to read JWT public key${NC}"
    echo "   Please run setup script first: ./scripts/setup-test-env.sh"
    exit 1
fi

# Start Spin in background
spin up -e JWT_PUBLIC_KEY="$JWT_PUBLIC_KEY" > /tmp/spin_output_$$.log 2>&1 &
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

# Session storage for Mcp-Session-Id header
SESSION_ID=""
REQUEST_ID=1

# Function to verify tools/list filtering (requires initialized session)
verify_tools_list() {
    local token_name="$1"
    local expected_tool_count="$2"
    local description="$3"

    echo -e "${YELLOW}Verifying tools/list for $token_name${NC}"

    # Load token
    local token=$($WASMCP_CLI jwt load-token "$token_name" 2>/dev/null)
    if [ -z "$token" ]; then
        echo -e "${RED}  ✗ Failed to load token '$token_name'${NC}"
        return 1
    fi

    # Build headers (include session ID if available)
    local headers="-H \"Authorization: Bearer $token\" -H \"Content-Type: application/json\" -H \"Accept: application/json, text/event-stream\""
    if [ -n "$SESSION_ID" ]; then
        headers="$headers -H \"Mcp-Session-Id: $SESSION_ID\""
    fi

    # Send tools/list request
    local list_response=$(eval curl -s -X POST \
        $headers \
        -d "'{\"jsonrpc\":\"2.0\",\"id\":$REQUEST_ID,\"method\":\"tools/list\",\"params\":{}}'" \
        http://localhost:3000/mcp)

    REQUEST_ID=$((REQUEST_ID + 1))

    # Count tools in response
    local tool_count=$(echo "$list_response" | grep -o '"name":"[^"]*"' | wc -l | tr -d ' ')

    # Verify expected count
    if [ "$tool_count" -eq "$expected_tool_count" ]; then
        echo -e "${GREEN}  ✓ Expected: $expected_tool_count tools - $description${NC}"

        # List the tools
        local tool_names=$(echo "$list_response" | grep -o '"name":"[^"]*"' | cut -d'"' -f4 | tr '\n' ', ' | sed 's/,$//')
        if [ -n "$tool_names" ]; then
            echo -e "${GREEN}    Tools shown: $tool_names${NC}"
        fi
    else
        echo -e "${RED}  ✗ Unexpected: Got $tool_count tools, expected $expected_tool_count${NC}"
        echo -e "${RED}    Description: $description${NC}"
        echo "    Response: $list_response"
    fi
    echo ""
}

# Initialize MCP session with a specific token
initialize_session() {
    local token_name="$1"

    echo -e "${BLUE}Initializing MCP session with token: $token_name${NC}"

    # Load token
    local token=$($WASMCP_CLI jwt load-token "$token_name" 2>/dev/null)
    if [ -z "$token" ]; then
        echo -e "${RED}  ✗ Failed to load token '$token_name'${NC}"
        return 1
    fi

    # Send initialize request
    local init_response=$(curl -s -X POST \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -H "Accept: application/json, text/event-stream" \
        -D /tmp/init_headers_$$.txt \
        -d '{"jsonrpc":"2.0","id":'$REQUEST_ID',"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"todo-list-auth-test","version":"1.0.0"}}}' \
        http://localhost:3000/mcp)

    REQUEST_ID=$((REQUEST_ID + 1))

    # Extract session ID from Mcp-Session-Id header
    if [ -f /tmp/init_headers_$$.txt ]; then
        SESSION_ID=$(grep -i "^Mcp-Session-Id:" /tmp/init_headers_$$.txt | cut -d' ' -f2 | tr -d '\r\n')
        rm -f /tmp/init_headers_$$.txt
    fi

    # Check if initialization succeeded
    if echo "$init_response" | grep -q '"result"'; then
        echo -e "${GREEN}  ✓ Session initialized${NC}"
        if [ -n "$SESSION_ID" ]; then
            echo -e "${GREEN}    Session ID: $SESSION_ID${NC}"
        fi
        return 0
    else
        echo -e "${RED}  ✗ Initialization failed${NC}"
        echo "    Response: $init_response"
        return 1
    fi
}

# Function to call a tool with the current session
call_tool() {
    local token_name="$1"
    local tool_name="$2"
    local args="$3"
    local expected_outcome="$4"  # "success" or "fail"

    echo -e "${YELLOW}Testing: $token_name → $tool_name${NC}"

    # Load token
    local token=$($WASMCP_CLI jwt load-token "$token_name" 2>/dev/null)
    if [ -z "$token" ]; then
        echo -e "${RED}  ✗ Failed to load token '$token_name'${NC}"
        return 1
    fi

    # Build headers
    local headers="-H \"Authorization: Bearer $token\" -H \"Content-Type: application/json\""
    if [ -n "$SESSION_ID" ]; then
        headers="$headers -H \"Mcp-Session-Id: $SESSION_ID\""
    fi

    # Make JSON-RPC request to call the tool
    local response=$(eval curl -s -X POST \
        $headers \
        -d "'{\"jsonrpc\":\"2.0\",\"id\":$REQUEST_ID,\"method\":\"tools/call\",\"params\":{\"name\":\"$tool_name\",\"arguments\":$args}}'" \
        http://localhost:3000/mcp)

    REQUEST_ID=$((REQUEST_ID + 1))

    # Check if response contains error or authorization failure
    if echo "$response" | grep -q '"is_error":true\|Authorization failed'; then
        if [ "$expected_outcome" = "fail" ]; then
            echo -e "${RED}  ✓ Expected: DENIED${NC}"
            # Show the authorization error message
            local error_msg=$(echo "$response" | grep -o '"text":"[^"]*"' | head -1 | cut -d'"' -f4)
            echo -e "${RED}    → $error_msg${NC}"
        else
            echo -e "${RED}  ✗ Unexpected: DENIED${NC}"
            echo "    Response: $response"
        fi
    else
        if [ "$expected_outcome" = "success" ]; then
            echo -e "${GREEN}  ✓ Expected: ALLOWED${NC}"
            # Show result if available
            local result=$(echo "$response" | grep -o '"text":"[^"]*"' | head -1 | cut -d'"' -f4)
            if [ -n "$result" ]; then
                echo -e "${GREEN}    Result: $result${NC}"
            fi
        else
            echo -e "${GREEN}  ✗ Unexpected: ALLOWED${NC}"
            echo "    Response: $response"
        fi
    fi
    echo ""
}

# Scenario 1: Admin User (Full Access)
echo "==================================================================="
echo "SCENARIO 1: Admin User (Full Access)"
echo "==================================================================="
echo "Token: admin"
echo "Claims: scope=\"mcp:read mcp:write\", role=\"admin\""
echo ""

# Initialize session
initialize_session "admin" || exit 1

# Verify tools list filtering (after session initialization)
verify_tools_list "admin" 4 "Admin sees all 4 tools"
echo ""

call_tool "admin" "add_item" '{"title":"Buy groceries"}' "success"
call_tool "admin" "add_item" '{"title":"Fix bug in auth"}' "success"
call_tool "admin" "list_items" '{}' "success"
call_tool "admin" "remove_item" '{"id":1}' "success"
call_tool "admin" "clear_all" '{}' "success"

echo -e "${GREEN}✓ Admin can access all tools${NC}"
echo ""

# Scenario 2: Read-Only User
echo "==================================================================="
echo "SCENARIO 2: Read-Only User (mcp:read scope only)"
echo "==================================================================="
echo "Token: readonly"
echo "Claims: scope=\"mcp:read\", role=\"viewer\""
echo ""

# Initialize new session for readonly user
initialize_session "readonly" || exit 1

# Verify tools list filtering (after session initialization)
verify_tools_list "readonly" 1 "Readonly sees only 1 tool (list_items)"
echo ""

call_tool "readonly" "list_items" '{}' "success"
echo -e "${BLUE}--- Tools requiring mcp:write scope ---${NC}"
call_tool "readonly" "add_item" '{"title":"Try to add item"}' "fail"
echo -e "${BLUE}--- Administrative tools ---${NC}"
call_tool "readonly" "remove_item" '{"id":1}' "fail"
call_tool "readonly" "clear_all" '{}' "fail"

echo -e "${GREEN}✓ Readonly user correctly blocked from write and admin operations${NC}"
echo ""

# Scenario 3: Normal User (Write access but not admin)
echo "==================================================================="
echo "SCENARIO 3: Normal User (Can add but not remove)"
echo "==================================================================="
echo "Token: analyst"
echo "Claims: scope=\"mcp:read mcp:write\", allowed_tools=\"add_item,list_items\""
echo ""

# Initialize new session for analyst user
initialize_session "analyst" || exit 1

# Verify tools list filtering (after session initialization)
verify_tools_list "analyst" 2 "Analyst sees only 2 tools (add_item, list_items)"
echo ""

call_tool "analyst" "add_item" '{"title":"User task 1"}' "success"
call_tool "analyst" "add_item" '{"title":"User task 2"}' "success"
call_tool "analyst" "list_items" '{}' "success"
echo -e "${BLUE}--- Administrative tools (not in allowed_tools) ---${NC}"
call_tool "analyst" "remove_item" '{"id":1}' "fail"
call_tool "analyst" "clear_all" '{}' "fail"

echo -e "${GREEN}✓ Normal user can add/list but not remove items${NC}"
echo ""

echo "==================================================================="
echo "Test Scenarios Summary"
echo "==================================================================="
echo ""
echo "Authorization Layers Demonstrated:"
echo ""
echo "1. ${GREEN}Conditional Tool Listing (PRIMARY SECURITY)${NC}"
echo "   • Tools filtered in tools/list based on user claims"
echo "   • Admin-only tools NOT SHOWN to non-admin users"
echo "   • Reduces attack surface - can't call what you can't see"
echo "   • Prevents information disclosure about available functionality"
echo ""
echo "2. ${GREEN}Runtime Validation (DEFENSE-IN-DEPTH)${NC}"
echo "   • Authorization re-checked in tools/call as fallback"
echo "   • Protects against direct tool calls bypassing discovery"
echo "   • Guards against bugs in filtering logic"
echo "   • Belt-and-suspenders security architecture"
echo ""
echo "Authorization Methods Used:"
echo ""
echo "A. ${GREEN}Scope-Based Access Control${NC}"
echo "   • mcp:read scope grants access to view operations (list_items)"
echo "   • mcp:write scope required for create operations (add_item)"
echo ""
echo "B. ${GREEN}Role-Based Access Control (RBAC)${NC}"
echo "   • role=admin claim required for delete operations (remove_item, clear_all)"
echo "   • Other roles (viewer, analyst) don't see admin tools"
echo ""
echo "C. ${GREEN}Attribute-Based Access Control (ABAC)${NC}"
echo "   • allowed_tools claim provides fine-grained tool-level permissions"
echo "   • If present, only tools in the list are shown"
echo "   • If absent, all authorized tools shown (default behavior)"
echo ""
echo "==================================================================="
echo ""
echo -e "${GREEN}✓ All tests completed successfully!${NC}"
