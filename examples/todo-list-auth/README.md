# Todo List with Authorization Patterns

This example demonstrates different authorization strategies for MCP tools using JWT-based authentication with session-based state persistence. It shows how to implement scope-based, role-based, and attribute-based access control in WebAssembly components.

## Overview

The todo-list-auth component provides four todo list operations, each with different authorization requirements:

| Tool | Authorization Required | Description |
|------|----------------------|-------------|
| `add_item` | `mcp:write` scope | Add a new todo item |
| `list_items` | `mcp:read` scope | View all todo items |
| `remove_item` | `role=admin` claim | Remove a todo item (admin only) |
| `clear_all` | `role=admin` claim | Clear all todo items (admin only) |

## Authorization Patterns

### 1. Scope-Based Access Control (SBAC)

Uses OAuth 2.0 scopes to control access to read and write operations:

- **`mcp:read`**: Grants access to read-only operations (list_items)
- **`mcp:write`**: Grants access to state-modifying operations (add_item)

```rust
if !check_scope(claims, "mcp:read") {
    return Ok(Some(auth_error("list_items", "mcp:read scope required")));
}
```

### 2. Role-Based Access Control (RBAC)

Uses custom JWT claims to restrict administrative operations:

- **`role=admin`**: Required for administrative tools (remove_item, clear_all)
- Other roles (viewer, analyst) cannot access admin tools

```rust
if !check_role(claims, "admin") {
    return Ok(Some(auth_error("remove_item", "role=admin claim required")));
}
```

### 3. Attribute-Based Access Control (ABAC)

Uses the `allowed_tools` claim for fine-grained tool-level permissions:

- If `allowed_tools` claim is present, only those tools are accessible
- If `allowed_tools` claim is absent, all tools are allowed (default)

```rust
fn check_tool_allowed(claims: Option<&JwtClaims>, tool_name: &str) -> bool {
    match claims {
        Some(c) => {
            match bindings::wasmcp::auth::helpers::get_claim(c, "allowed_tools") {
                Some(allowed) => {
                    // Parse comma-separated list
                    allowed.split(',').any(|t| t.trim() == tool_name)
                }
                None => true, // No allowed_tools claim means allow all
            }
        }
        None => false, // No claims = no authorization
    }
}
```

## Session-Based State Management

This example uses session storage to persist todo lists across requests:

- Each session maintains its own isolated todo list
- State is stored in the key-value store using `wasmcp:keyvalue/store`
- Session IDs are managed via `Mcp-Session-Id` headers
- Session hijacking protection validates JWT identity matches session-bound identity

```rust
// Load todo list from session storage
fn load_todo_list(session: &Session) -> Vec<TodoItem> {
    let session_obj = sessions::Session::open(&session.session_id, &session.store_id)?;
    match session_obj.get("todo:list") {
        Ok(Some(TypedValue::AsBytes(bytes))) => serde_json::from_slice(&bytes).unwrap_or_default(),
        _ => Vec::new(),
    }
}
```

## Building

```bash
# Install required tools
make setup

# Update WIT dependencies
make wit

# Build the component
make build

# Compose into MCP server (adds transport layer)
make compose
```

This creates two files:
- `target/wasm32-wasip2/release/todo_list_auth.wasm` - The component with authorization logic
- `mcp-server.wasm` - The composed MCP server (component + transport layer)

## Running with Spin

This example uses [Spin](https://developer.fermyon.com/spin) to run the MCP server with HTTP transport.

### MCP Protocol Lifecycle

The Model Context Protocol (June 2025 spec) uses JSON-RPC 2.0 over HTTP with the following lifecycle:

1. **initialize** - Client sends `initialize` request with protocol version and capabilities
   - Server responds with its capabilities (tools, resources, prompts)
   - Server includes `Mcp-Session-Id` header for session management

2. **initialized** (optional) - Client sends notification confirming initialization complete

3. **Normal operations** - Client makes requests like:
   - `tools/list` - List available tools
   - `tools/call` - Execute a tool with arguments
   - All subsequent requests must include `Mcp-Session-Id` header from initialize response

4. **Session management** - Session ID binds to the JWT identity to prevent session hijacking

All requests must include the `Authorization: Bearer <JWT>` header for authentication.

### Quick Start

```bash
# 1. Build and compose the server
make work  # Builds component + composes into MCP server

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

### What `make work` does

1. **Builds** the todo-list-auth component (`todo_list_auth.wasm`)
2. **Composes** it into a complete MCP server using `wasmcp compose server`:
   - Adds HTTP transport layer with session support
   - Adds session store (for stateful operations)
   - Adds terminal handler (method-not-found)
   - Creates `mcp-server.wasm` ready to run with Spin

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

### Testing with Different Tokens

MCP follows a request/response lifecycle. First initialize the session, then make tool calls:

```bash
# 1. Initialize session with admin token (returns session ID in Mcp-Session-Id header)
TOKEN=$(../../cli/target/aarch64-apple-darwin/release/wasmcp jwt load-token admin)
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
TOKEN=$(../../cli/target/aarch64-apple-darwin/release/wasmcp jwt load-token readonly)
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
- ✅ Can call add_item (mcp:write)
- ✅ Can call list_items (mcp:read)
- ✅ Can call remove_item (role=admin)
- ✅ Can call clear_all (role=admin)

### Scenario 2: Read-Only User

**Token**: `readonly`
**Claims**: `scope="mcp:read"`, `role="viewer"`

**Expected Behavior**:
- ❌ Cannot call add_item (requires mcp:write)
- ✅ Can call list_items (mcp:read)
- ❌ Cannot call remove_item (requires role=admin)
- ❌ Cannot call clear_all (requires role=admin)

**Error Example**:
```
Authorization failed for 'add_item': mcp:write scope required
```

### Scenario 3: Analyst User (ABAC)

**Token**: `analyst`
**Claims**: `scope="mcp:read mcp:write"`, `allowed_tools="add_item,list_items"`

**Expected Behavior**:
- ✅ Can call add_item (in allowed_tools list and has mcp:write)
- ✅ Can call list_items (in allowed_tools list and has mcp:read)
- ❌ Cannot call remove_item (not in allowed_tools list)
- ❌ Cannot call clear_all (not in allowed_tools list)

**Error Example**:
```
Authorization failed for 'remove_item': Tool not in allowed_tools list
```

## Integration with Claude Code

Configure Claude Code to use different authorization contexts by creating multiple MCP server configurations:

```json
{
  "mcpServers": {
    "todo-admin": {
      "command": "spin",
      "args": [
        "up",
        "--listen",
        "127.0.0.1:3001",
        "--from",
        "/path/to/todo-list-auth"
      ],
      "env": {
        "JWT_PUBLIC_KEY": "$(cat ~/.wasmcp/jwt-test/public.pem)"
      }
    },
    "todo-readonly": {
      "command": "spin",
      "args": [
        "up",
        "--listen",
        "127.0.0.1:3002",
        "--from",
        "/path/to/todo-list-auth"
      ],
      "env": {
        "JWT_PUBLIC_KEY": "$(cat ~/.wasmcp/jwt-test/public.pem)"
      }
    }
  }
}
```

Each Claude Code instance would then include the appropriate JWT token in the `Authorization` header when making requests.

Or run Spin manually with different configurations:

```bash
# Terminal 1: Admin server
export JWT_PUBLIC_KEY="$(cat ~/.wasmcp/jwt-test/public.pem)"
spin up --listen 127.0.0.1:3001 -e JWT_PUBLIC_KEY="$JWT_PUBLIC_KEY"

# Terminal 2: Readonly server
export JWT_PUBLIC_KEY="$(cat ~/.wasmcp/jwt-test/public.pem)"
spin up --listen 127.0.0.1:3002 -e JWT_PUBLIC_KEY="$JWT_PUBLIC_KEY"
```

## JWT Commands Reference

The `wasmcp jwt` commands used in this example:

```bash
# Generate RSA keypair for testing
wasmcp jwt generate-keypair

# Create a custom token
wasmcp jwt mint \
  --subject "user@example.com" \
  --audience "http://localhost:3000" \
  --scope "mcp:read mcp:write" \
  --claim role=analyst \
  --claim allowed_tools="add_item,list_items" \
  --save-as my-token

# List all stored tokens
wasmcp jwt list-tokens

# Load a token for use
wasmcp jwt load-token my-token

# Decode and inspect a token
wasmcp jwt decode-token my-token
```

## Using These Authorization Patterns in Your Components

### Pattern 1: Scope-Based Access Control (SBAC)

Implement scope-based access control in your tools:

```rust
use bindings::wasmcp::auth::helpers;
use bindings::wasmcp::mcp_v20250618::server_handler::MessageContext;

fn call_tool(
    ctx: MessageContext,
    request: CallToolRequest,
) -> Result<Option<CallToolResult>, ErrorCode> {
    // Extract JWT claims from context
    let claims = ctx.identity.as_ref().map(|id| &id.claims);

    match request.name.as_str() {
        "read_operation" => {
            // Check for mcp:read scope
            if !check_scope(claims, "mcp:read") {
                return Ok(Some(error_result(
                    "Authorization failed: mcp:read scope required".to_string()
                )));
            }
            // Execute read operation...
        }
        "write_operation" => {
            // Check for mcp:write scope
            if !check_scope(claims, "mcp:write") {
                return Ok(Some(error_result(
                    "Authorization failed: mcp:write scope required".to_string()
                )));
            }
            // Execute write operation...
        }
        _ => Ok(None),
    }
}

fn check_scope(claims: Option<&JwtClaims>, required: &str) -> bool {
    match claims {
        Some(c) => helpers::has_scope(c, required),
        None => false,
    }
}
```

**JWT Claims** (standard OAuth 2.0 scope claim):
```json
{
  "scope": "mcp:read mcp:write"
}
```

### Pattern 2: Role-Based Access Control (RBAC)

Use custom claims for role-based authorization:

```rust
fn check_role(claims: Option<&JwtClaims>, required_role: &str) -> bool {
    match claims {
        Some(c) => {
            match helpers::get_claim(c, "role") {
                Some(role) => role == required_role,
                None => false,
            }
        }
        None => false,
    }
}

// In your tool handler
fn call_tool(ctx: MessageContext, request: CallToolRequest) -> Result<...> {
    let claims = ctx.identity.as_ref().map(|id| &id.claims);

    match request.name.as_str() {
        "admin_operation" => {
            if !check_role(claims, "admin") {
                return Ok(Some(error_result(
                    "Authorization failed: role=admin required".to_string()
                )));
            }
            // Execute admin operation...
        }
        "analyst_operation" => {
            if !check_role(claims, "analyst") {
                return Ok(Some(error_result(
                    "Authorization failed: role=analyst required".to_string()
                )));
            }
            // Execute analyst operation...
        }
        _ => Ok(None),
    }
}
```

**JWT Claims** (custom role claim):
```json
{
  "role": "admin",
  "scope": "mcp:read mcp:write"
}
```

### Pattern 3: Attribute-Based Access Control (ABAC)

Implement fine-grained tool-level permissions:

```rust
fn check_tool_allowed(claims: Option<&JwtClaims>, tool_name: &str) -> bool {
    match claims {
        Some(c) => {
            match helpers::get_claim(c, "allowed_tools") {
                Some(allowed) => {
                    // Parse comma-separated list
                    allowed.split(',')
                        .map(|s| s.trim())
                        .any(|t| t == tool_name)
                }
                None => true, // No allowed_tools claim = allow all
            }
        }
        None => false,
    }
}

// Check in tool handler
fn call_tool(ctx: MessageContext, request: CallToolRequest) -> Result<...> {
    let claims = ctx.identity.as_ref().map(|id| &id.claims);

    // Check if tool is allowed via ABAC
    if !check_tool_allowed(claims, &request.name) {
        return Ok(Some(error_result(
            format!("Authorization failed: Tool '{}' not in allowed_tools list", request.name)
        )));
    }

    // Also check required scopes/roles for specific tools
    match request.name.as_str() {
        "sensitive_operation" => {
            if !check_scope(claims, "mcp:write") {
                return Ok(Some(error_result(
                    "Authorization failed: mcp:write scope required".to_string()
                )));
            }
            // Execute operation...
        }
        _ => Ok(None),
    }
}
```

**JWT Claims** (custom allowed_tools claim):
```json
{
  "scope": "mcp:read mcp:write",
  "role": "analyst",
  "allowed_tools": "add_item,list_items,generate_report"
}
```

### Pattern 4: Layered Authorization

Combine all three patterns for defense in depth:

```rust
fn call_tool(ctx: MessageContext, request: CallToolRequest) -> Result<...> {
    let claims = ctx.identity.as_ref().map(|id| &id.claims);

    // Layer 1: ABAC - Check allowed_tools list
    if !check_tool_allowed(claims, &request.name) {
        return Ok(Some(error_result(
            format!("Tool '{}' not in allowed_tools list", request.name)
        )));
    }

    // Layer 2: SBAC - Check required scope
    match request.name.as_str() {
        "read_data" | "list_items" => {
            if !check_scope(claims, "mcp:read") {
                return Ok(Some(error_result("mcp:read scope required".to_string())));
            }
        }
        "write_data" | "add_item" => {
            if !check_scope(claims, "mcp:write") {
                return Ok(Some(error_result("mcp:write scope required".to_string())));
            }
        }
        _ => {}
    }

    // Layer 3: RBAC - Check role for admin operations
    match request.name.as_str() {
        "delete_data" | "clear_all" => {
            if !check_role(claims, "admin") {
                return Ok(Some(error_result("role=admin required".to_string())));
            }
        }
        _ => {}
    }

    // All checks passed - execute tool
    execute_tool(&ctx, &request)
}
```

### Using Session-Based State in Your Components

Session storage provides persistent key-value storage scoped to a client session:

```rust
use bindings::wasmcp::mcp_v20250618::sessions::Session;
use bindings::wasmcp::keyvalue::store::TypedValue;

fn save_data(ctx: &MessageContext, key: &str, value: &str) -> Result<(), ErrorCode> {
    let session_info = ctx.session.as_ref()
        .ok_or(ErrorCode::InvalidRequest)?;

    let session = Session::open(&session_info.session_id, &session_info.store_id)?;
    session.set(key, &TypedValue::AsString(value.to_string()))?;
    Ok(())
}

fn load_data(ctx: &MessageContext, key: &str) -> Result<Option<String>, ErrorCode> {
    let session_info = ctx.session.as_ref()
        .ok_or(ErrorCode::InvalidRequest)?;

    let session = Session::open(&session_info.session_id, &session_info.store_id)?;
    match session.get(key)? {
        Some(TypedValue::AsString(s)) => Ok(Some(s)),
        Some(TypedValue::AsBytes(b)) => Ok(Some(String::from_utf8_lossy(&b).to_string())),
        None => Ok(None),
    }
}
```

### Session Hijacking Protection

The transport layer automatically validates that JWT identity matches the session-bound identity:

1. On session initialization, `jwt:sub` and `jwt:iss` claims are stored
2. On subsequent requests with session ID, these stored values are compared with current JWT
3. Mismatches result in HTTP 403 rejection

**No additional code required** - this protection is built into the transport layer.

## Implementation Details

### MessageContext and Session

The component receives JWT claims and session information through the `MessageContext` parameter:

```rust
fn call_tool(
    ctx: MessageContext,
    request: CallToolRequest,
) -> Result<Option<CallToolResult>, ErrorCode> {
    // Extract JWT claims from identity (if present)
    let claims = ctx.identity.as_ref().map(|id| &id.claims);

    // Get user identifier from JWT subject claim
    let user = ctx
        .identity
        .as_ref()
        .and_then(|id| bindings::wasmcp::auth::helpers::get_claim(&id.claims, "sub"))
        .unwrap_or_else(|| "anonymous".to_string());

    // Get session - required for state persistence
    let session = match &ctx.session {
        Some(s) => s,
        None => {
            return Ok(Some(error_result(
                "Session required for todo operations".to_string(),
            )));
        }
    };

    // Check authorization and execute tool
    // ...
}
```

### Authorization Helpers

Three helper functions from `wasmcp:auth/helpers` implement the authorization checks:

- `has_scope(&claims, scope)`: Check for OAuth scope
- `get_claim(&claims, "role")`: Get custom claim value for role check
- `get_claim(&claims, "allowed_tools")`: Get custom claim for ABAC

### State Management with Session Storage

Todo items are persisted in session storage using the key-value store:

```rust
// Save todo list to session storage
fn save_todo_list(session: &Session, list: &[TodoItem]) {
    if let Ok(session_obj) = sessions::Session::open(&session.session_id, &session.store_id) {
        if let Ok(json_bytes) = serde_json::to_vec(list) {
            let _ = session_obj.set("todo:list", &TypedValue::AsBytes(json_bytes));
        }
    }
}

// Get next ID from session storage
fn get_next_id(session: &Session) -> usize {
    let session_obj = sessions::Session::open(&session.session_id, &session.store_id)?;
    match session_obj.get("todo:next_id") {
        Ok(Some(TypedValue::AsString(id_str))) => id_str.parse().unwrap_or(1),
        _ => 1,
    }
}
```

### Session Hijacking Protection

The transport layer validates that the JWT identity matches the session-bound identity:

1. On session initialization, `jwt:sub` and `jwt:iss` claims are stored in session
2. On subsequent requests with session ID, these stored values are compared with current JWT
3. If they don't match, the request is rejected with HTTP 403

This prevents an attacker from using a different valid JWT with a stolen session ID.

## Security Considerations

**⚠️ FOR LOCAL TESTING ONLY**

The JWT infrastructure in this example is designed for local development and testing:

- Test keypairs are stored in `~/.wasmcp/jwt-test/`
- Private keys have 0600 permissions (Unix only)
- Tokens are stored unencrypted in `~/.wasmcp/jwt-test/tokens/`

**For production use**, you should:
- Use a proper identity provider (OAuth 2.0 / OIDC)
- Implement token refresh and rotation
- Store secrets in secure vaults
- Use proper key management systems
- Implement audit logging
- Enable HTTPS/TLS for all communication
- Use secure session storage backends

## Files

```
todo-list-auth/
├── Cargo.toml              # Rust package configuration
├── Makefile                # Build targets
├── README.md               # This file
├── spin.toml               # Spin application configuration
├── .gitignore              # Git ignore rules
├── wit/                    # WIT dependencies
│   ├── deps/
│   ├── deps.lock
│   ├── deps.toml
│   └── world.wit           # Component world definition (todo-list)
├── src/
│   └── lib.rs              # Component implementation with auth logic
└── scripts/
    ├── setup-test-env.sh   # Generate keys and test tokens
    └── test-scenarios.sh   # Run authorization test scenarios
```

## Related Documentation

- [JWT Testing Infrastructure](../../.agent/jwt-plan.md)
- [wasmcp JWT Commands](../../cli/README.md#jwt-commands)
- [MCP Authorization Specification](https://spec.modelcontextprotocol.io/authorization/)
- [WebAssembly Component Model](https://component-model.bytecodealliance.org/)
