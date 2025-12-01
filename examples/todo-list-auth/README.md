# Todo List with Authorization Patterns

This example demonstrates different authorization strategies for MCP tools using JWT-based authentication with session-based state persistence. It shows how to implement scope-based, role-based, and attribute-based access control in wasmcp components.

## Overview

The todo-list-auth component provides four todo list operations, each with different authorization requirements:

| Tool | Authorization Required | Description |
|------|----------------------|-------------|
| `add_item` | `mcp:write` scope | Add a new todo item |
| `list_items` | `mcp:read` scope | View all todo items |
| `remove_item` | `role=admin` claim | Remove a todo item (admin only) |
| `clear_all` | `role=admin` claim | Clear all todo items (admin only) |

## Quick Start

```bash
# 1. Build and compose the server
make work 

# 2. Set up test environment (generates keys and tokens)
./scripts/setup-test-env.sh

# 3. Run test scenarios
./scripts/test-scenarios.sh
```

The test script will:
- Start Spin server with proper configuration
- Initialize sessions for different user roles
- Test all authorization patterns
- Clean up processes when done

## Authorization Patterns

This example demonstrates **two complementary authorization layers** working together:

### Layer 1: Conditional Tool Listing (PRIMARY)

**Security through elimination**: Tools are filtered in `list_tools()` before being shown to the client.

**Benefits**:
- Reduces attack surface - unauthorized tools never appear to the client
- Prevents information disclosure about available functionality
- Cleaner user experience - users only see what they can use

**Implementation**:
```rust
fn list_tools(ctx: MessageContext, _request: ListToolsRequest) -> Result<ListToolsResult, ErrorCode> {
    let claims = ctx.identity.as_ref().map(|id| &id.claims);

    // Filter tools based on user's claims
    let filtered_tools: Vec<Tool> = all_tools
        .into_iter()
        .filter(|tool| should_show_tool(claims, &tool.name))
        .collect();

    Ok(ListToolsResult { tools: filtered_tools, ... })
}
```

**Authorization logic in `should_show_tool()`**:
1. **Scope-based**: OAuth 2.0 scopes (mcp:read, mcp:write)
2. **Role-based**: Custom claims (role=admin)
3. **Attribute-based**: Fine-grained control (allowed_tools claim)

### Layer 2: Runtime Validation (DEFENSE-IN-DEPTH)

**Belt-and-suspenders approach**: Even though tools are filtered at listing time, authorization is re-validated in `call_tool()`.

**Implementation**:
```rust
fn call_tool(ctx: MessageContext, request: CallToolRequest) -> Result<Option<CallToolResult>, ErrorCode> {
    // Defense-in-depth: Re-validate authorization even though we filtered in list_tools()
    let claims = ctx.identity.as_ref().map(|id| &id.claims);

    match request.name.as_str() {
        "add_item" => {
            if !check_scope(claims, "mcp:write") {
                return Ok(Some(auth_error("add_item", "mcp:write scope required")));
            }
            // ... execute tool
        }
        "remove_item" => {
            if !check_role(claims, "admin") {
                return Ok(Some(auth_error("remove_item", "role=admin claim required")));
            }
            // ... execute tool
        }
        // ...
    }
}
```

### Authorization Methods

#### 1. Scope-Based Access Control (OAuth 2.0)

Controls access to read and write operations:

- **`mcp:read`**: Read-only operations (list_items)
- **`mcp:write`**: State-modifying operations (add_item)

```rust
if !check_scope(claims, "mcp:read") {
    return false; // Don't show tool
}
```

#### 2. Role-Based Access Control (RBAC)

Restricts administrative operations via custom JWT claims:

- **`role=admin`**: Administrative tools (remove_item, clear_all)
- Other roles (viewer, analyst) don't see admin tools

```rust
if !check_role(claims, "admin") {
    return false; // Don't show tool
}
```

#### 3. Attribute-Based Access Control (ABAC)

Fine-grained tool-level permissions via `allowed_tools` claim:

- If `allowed_tools` claim is present, only listed tools are shown
- If `allowed_tools` claim is absent, all authorized tools are shown (default)

```rust
fn check_tool_allowed(claims: Option<&JwtClaims>, tool_name: &str) -> bool {
    match claims {
        Some(c) => {
            match bindings::wasmcp::auth::helpers::get_claim(c, "allowed_tools") {
                Some(allowed) => allowed.split(',').any(|t| t.trim() == tool_name),
                None => true, // No restriction
            }
        }
        None => false, // No claims = no authorization
    }
}
```

## Testing

### 1. Set Up Test Environment

Generate RSA keypair and create test tokens:

```bash
./scripts/setup-test-env.sh
```

This creates three test tokens with audience `http://localhost:3000`:

- **admin**: Full access (scopes: `mcp:read mcp:write`, claim: `role=admin`)
- **readonly**: Read-only access (scope: `mcp:read`, claim: `role=viewer`)
- **analyst**: Limited tools (scopes: `mcp:read mcp:write`, claim: `allowed_tools="add_item,list_items"`)

### 2. Run Test Scenarios

```bash
./scripts/test-scenarios.sh
```

This demonstrates all three authorization patterns with expected outcomes.

## Test Scenarios

### Scenario 1: Admin User (Full Access)

**Token**: `admin`
**Claims**: `scope="mcp:read mcp:write"`, `role="admin"`

**Expected Behavior**:
- **tools/list**: Shows all 4 tools (add_item, list_items, remove_item, clear_all)
- ✅ Can call add_item (mcp:write)
- ✅ Can call list_items (mcp:read)
- ✅ Can call remove_item (role=admin)
- ✅ Can call clear_all (role=admin)

### Scenario 2: Read-Only User

**Token**: `readonly`
**Claims**: `scope="mcp:read"`, `role="viewer"`

**Expected Behavior**:
- **tools/list**: Shows only 1 tool (list_items)
  - ❌ add_item NOT SHOWN (requires mcp:write)
  - ❌ remove_item NOT SHOWN (requires role=admin)
  - ❌ clear_all NOT SHOWN (requires role=admin)
- ✅ Can call list_items (mcp:read)

**Security Note**: Admin tools (`remove_item`, `clear_all`) are completely hidden from read-only users. They don't appear in the tools list, eliminating them from the attack surface.

### Scenario 3: Analyst User (ABAC)

**Token**: `analyst`
**Claims**: `scope="mcp:read mcp:write"`, `allowed_tools="add_item,list_items"`

**Expected Behavior**:
- **tools/list**: Shows only 2 tools (add_item, list_items)
  - ❌ remove_item NOT SHOWN (not in allowed_tools)
  - ❌ clear_all NOT SHOWN (not in allowed_tools)
- ✅ Can call add_item (in allowed_tools list and has mcp:write)
- ✅ Can call list_items (in allowed_tools list and has mcp:read)

**Security Note**: Even though analyst has mcp:read and mcp:write scopes, they only see the 2 tools explicitly listed in their `allowed_tools` claim. This demonstrates fine-grained ABAC control.

### Manual Testing

If you want to test manually:

```bash
# 1. Build and compose
make work

# 2. Set up test environment
./scripts/setup-test-env.sh

# 3. Start Spin server
JWT_PUBLIC_KEY=$(cat ~/.wasmcp/jwt-test/public.pem)
spin up -e JWT_PUBLIC_KEY="$JWT_PUBLIC_KEY"
```

Server will be available at `http://localhost:3000/mcp`.

#### Testing with Different Tokens

You can use the following to manually show the difference in functionality based on roles/scopes 

```bash
# 1. Initialize session with admin token (returns session ID in Mcp-Session-Id header)
TOKEN=$(wasmcp jwt load-token admin)
SESSION_ID=$(curl -s -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -D - \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}}}' \
  http://localhost:3000/mcp | grep -i "mcp-session-id" | cut -d' ' -f2 | tr -d '\r')

# 2. Add a todo item (requires mcp:write scope)
curl -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"add_item","arguments":"{\"title\":\"Write documentation\"}"}}' \
  http://localhost:3000/mcp

# 3. List todo items (requires mcp:read scope)
curl -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_items","arguments":"{}"}}' \
  http://localhost:3000/mcp

# 4. Remove a todo item (requires role=admin)
curl -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"remove_item","arguments":"{\"id\":1}"}}' \
  http://localhost:3000/mcp

# Test authorization denial with readonly token trying to add item (requires mcp:write)
TOKEN=$(wasmcp jwt load-token readonly)
SESSION_ID=$(curl -s -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -D - \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}}}' \
  http://localhost:3000/mcp | grep -i "mcp-session-id" | cut -d' ' -f2 | tr -d '\r')

curl -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"add_item","arguments":"{\"title\":\"This should fail\"}"}}' \
  http://localhost:3000/mcp
# Expected: Authorization failed for 'add_item': mcp:write scope required
```

## Integration with Claude Code

Configure Claude Code to show different authorization contexts by creating multiple MCP server configurations:

```bash
export JWT_PUBLIC_KEY=$(cat ~/.wasmcp/jwt-test/public.pem)
export ADMIN_TOKEN=$(wasmcp jwt load-token admin)
export READ_TOKEN=$(wasmcp jwt load-token readonly)
spin up -f /path/to/spin.toml
```

```json
{
  "mcpServers": {
    "readonly": {
      "type": "http",
      "url": "http://127.0.0.1:3000/mcp",
      "headers": {
        "Authorization": "Bearer ${READ_TOKEN}"
      }
    },
    "admin": {
      "type": "http",
      "url": "http://127.0.0.1:3000/mcp/todo",
      "headers": {
        "Authorization": "Bearer ${ADMIN_TOKEN}"
      }
    }
  }
}
```

Each Claude Code instance sends the configured JWT token when making requests.
