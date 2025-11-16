# Sessions Guide

Complete guide to using stateful sessions in wasmcp MCP servers.

## What Are Sessions?

Sessions enable MCP servers to maintain state across multiple requests from the same client. Each session:

- Has a unique ID passed via `Mcp-Session-Id` HTTP header (or transport-specific mechanism)
- Stores key-value data that persists across requests
- Automatically initializes on first request
- Can be terminated by client or server
- Backed by WASI key-value store

**Use cases:**
- Multi-step workflows (wizards, forms)
- Shopping carts and order building
- Authentication state and user preferences
- Caching expensive computations
- Request context and history

## Key Naming and Organization

Sessions store key-value data with automatic isolation between sessions.

**Key Naming Guidelines:**

- **Use hierarchical names with colons** for related data:
  ```rust
  session.set("cart:items", &value)?;
  session.set("cart:total", &value)?;
  session.set("user:preferences:theme", &value)?;
  session.set("user:preferences:language", &value)?;
  ```

- **Use flat names** for simple cases:
  ```rust
  session.set("counter", &TypedValue::AsU64(1))?;
  session.set("authenticated", &TypedValue::AsBool(true))?;
  ```

**Restrictions:**
- Maximum key size: 1KB
- Keys are case-sensitive
- Reserved names: `__meta__`, `__metadata__`, `metadata`, `meta`

**Session Isolation:**
Each session's data is automatically isolated. Sessions cannot access each other's data, even if using identical key names.

> **Implementation Details:** For technical information about storage format, UUID validation, and isolation mechanisms, see [session-store component README](/crates/session-store/README.md).

## Session Lifecycle

### 1. Initialization

**Automatic (HTTP transport):**
```
Client → Request without Mcp-Session-Id header
Server → Creates new session, returns Mcp-Session-Id in response
Client → Subsequent requests include Mcp-Session-Id
```

**Manual (tool code):**
Sessions are automatically opened by the transport layer. Tools receive session info via `MessageContext`.

### 2. Opening a Session

Tools access sessions through `MessageContext`:

**Rust:**
```rust
use crate::bindings::exports::wasmcp::mcp_v20250618::tools::{
    Guest, CallToolRequest, CallToolResult, MessageContext
};
use crate::bindings::wasmcp::mcp_v20250618::sessions::Session;

fn call_tool(ctx: MessageContext, request: CallToolRequest) -> Result<CallToolResult, String> {
    // Check if session exists
    let Some(session_info) = ctx.session else {
        return Err("Session required".to_string());
    };

    // Open session
    let session = Session::open(&session_info.session_id, &session_info.store_id)
        .map_err(|e| format!("Failed to open session: {:?}", e))?;

    // Use session...
    Ok(result)
}
```

**Python:**
```python
from wasmcp.mcp_v20250618.tools import Guest, CallToolRequest, CallToolResult, MessageContext
from wasmcp.mcp_v20250618.sessions import Session

class MyTools(Guest):
    def call_tool(self, ctx: MessageContext, request: CallToolRequest) -> CallToolResult:
        if ctx.session is None:
            raise Exception("Session required")

        # Open session
        session = Session.open(ctx.session.session_id, ctx.session.store_id)

        # Use session...
        return result
```

### 3. Storing Data

Use `TypedValue` for type-safe storage:

**Rust:**
```rust
use crate::bindings::wasmcp::keyvalue::store::TypedValue;

// Store string
session.set("user_name", &TypedValue::AsString("Alice".to_string()))?;

// Store number
session.set("counter", &TypedValue::AsU64(42))?;

// Store JSON
session.set("cart", &TypedValue::AsJson(r#"{"items": []}"#.to_string()))?;

// Store boolean
session.set("premium", &TypedValue::AsBool(true))?;

// Store bytes
session.set("data", &TypedValue::AsBytes(vec![1, 2, 3]))?;
```

**Python:**
```python
from wasmcp.keyvalue.store import TypedValue

# Store string
session.set("user_name", TypedValue.AsString("Alice"))

# Store number
session.set("counter", TypedValue.AsU64(42))

# Store JSON
session.set("cart", TypedValue.AsJson('{"items": []}'))

# Store boolean
session.set("premium", TypedValue.AsBool(True))

# Store bytes
session.set("data", TypedValue.AsBytes(bytes([1, 2, 3])))
```

### 4. Retrieving Data

Pattern match on `TypedValue` variants:

**Rust:**
```rust
match session.get("counter")? {
    Some(TypedValue::AsU64(n)) => {
        println!("Counter: {}", n);
    }
    Some(TypedValue::AsString(s)) => {
        // Handle string that should be number
        let n = s.parse::<u64>().unwrap_or(0);
    }
    Some(other) => {
        return Err(format!("Unexpected type: {:?}", other));
    }
    None => {
        println!("Key not found, using default");
    }
}
```

**Python:**
```python
value = session.get("counter")
if value is None:
    count = 0
elif isinstance(value, TypedValue.AsU64):
    count = value.value
elif isinstance(value, TypedValue.AsString):
    count = int(value.value)
else:
    raise Exception(f"Unexpected type: {type(value)}")
```

### 5. Termination

**From tool code:**
```rust
// Mark session as terminated (soft delete)
session.terminate(Some("User logged out"))?;
```

**From client:**
```bash
# Client can request session termination via MCP protocol
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: session-uuid" \
  -d '{"jsonrpc": "2.0", "id": 1, "method": "session/terminate"}'
```

**Note:** Termination marks the session as ended but doesn't delete data. This allows for graceful cleanup and audit trails.

## TypedValue API

Sessions use `TypedValue` enum for type-safe storage:

```wit
variant typed-value {
    as-string(string),
    as-json(string),
    as-u64(u64),
    as-s64(s64),
    as-bool(bool),
    as-bytes(list<u8>),
}
```

**Benefits:**
- Runtime type checking
- Explicit serialization format
- Type hints for debugging
- Forward compatibility

**See:** [typed-value.md](./typed-value.md) for detailed API reference

## Session Patterns

### Pattern 1: Counter

Track tool invocations per session:

**Rust:**
```rust
fn increment_counter(session: &Session) -> Result<u64, String> {
    let current = match session.get("count")? {
        Some(TypedValue::AsU64(n)) => n,
        _ => 0,
    };

    let new_count = current + 1;
    session.set("count", &TypedValue::AsU64(new_count))?;

    Ok(new_count)
}
```

**Example:** See [examples/counter-middleware](../examples/counter-middleware/)

### Pattern 2: Shopping Cart

Build state across multiple tool calls:

**Rust:**
```rust
fn add_to_cart(session: &Session, item: &str) -> Result<(), String> {
    // Get current cart
    let mut cart: Vec<String> = match session.get("cart")? {
        Some(TypedValue::AsJson(json)) => {
            serde_json::from_str(&json).unwrap_or_default()
        }
        _ => Vec::new(),
    };

    // Add item
    cart.push(item.to_string());

    // Save back
    let json = serde_json::to_string(&cart).unwrap();
    session.set("cart", &TypedValue::AsJson(json))?;

    Ok(())
}

fn checkout(session: &Session) -> Result<Vec<String>, String> {
    let cart: Vec<String> = match session.get("cart")? {
        Some(TypedValue::AsJson(json)) => {
            serde_json::from_str(&json).unwrap_or_default()
        }
        _ => Vec::new(),
    };

    // Clear cart after checkout
    session.set("cart", &TypedValue::AsJson("[]".to_string()))?;

    Ok(cart)
}
```

### Pattern 3: Multi-Step Form

Collect data across multiple interactions:

**Rust:**
```rust
struct FormData {
    name: Option<String>,
    email: Option<String>,
    age: Option<u32>,
}

fn save_form_field(session: &Session, field: &str, value: &str) -> Result<(), String> {
    // Load current form data
    let mut form: FormData = match session.get("form")? {
        Some(TypedValue::AsJson(json)) => {
            serde_json::from_str(&json).unwrap_or_default()
        }
        _ => FormData::default(),
    };

    // Update field
    match field {
        "name" => form.name = Some(value.to_string()),
        "email" => form.email = Some(value.to_string()),
        "age" => form.age = value.parse().ok(),
        _ => return Err("Unknown field".to_string()),
    }

    // Save back
    let json = serde_json::to_string(&form).unwrap();
    session.set("form", &TypedValue::AsJson(json))?;

    Ok(())
}

fn submit_form(session: &Session) -> Result<FormData, String> {
    match session.get("form")? {
        Some(TypedValue::AsJson(json)) => {
            let form: FormData = serde_json::from_str(&json)
                .map_err(|e| format!("Invalid form data: {}", e))?;

            // Validate required fields
            if form.name.is_none() || form.email.is_none() {
                return Err("Missing required fields".to_string());
            }

            Ok(form)
        }
        _ => Err("No form data found".to_string()),
    }
}
```

### Pattern 4: Caching

Cache expensive computations:

**Rust:**
```rust
fn get_or_compute(session: &Session, key: &str, compute_fn: impl FnOnce() -> String)
    -> Result<String, String>
{
    // Check cache
    if let Some(TypedValue::AsString(cached)) = session.get(key)? {
        return Ok(cached);
    }

    // Compute
    let result = compute_fn();

    // Cache for next time
    session.set(key, &TypedValue::AsString(result.clone()))?;

    Ok(result)
}
```

### Pattern 5: User Preferences

Store user settings:

**Rust:**
```rust
fn save_preference(session: &Session, key: &str, value: &str) -> Result<(), String> {
    let pref_key = format!("pref:{}", key);
    session.set(&pref_key, &TypedValue::AsString(value.to_string()))?;
    Ok(())
}

fn get_preference(session: &Session, key: &str, default: &str) -> Result<String, String> {
    let pref_key = format!("pref:{}", key);
    match session.get(&pref_key)? {
        Some(TypedValue::AsString(value)) => Ok(value),
        _ => Ok(default.to_string()),
    }
}
```

## Session Metadata

Sessions store metadata automatically:

**Internal structure (not directly accessible):**
```json
{
  "__meta__": {
    "created_at": 1234567890,
    "terminated": false,
    "termination_reason": null,
    "expires_at": null
  },
  "data": {
    "user_name": {"type": "string", "value": "Alice"},
    "counter": {"type": "u64", "value": 42}
  }
}
```

**Accessing metadata:**
```rust
// Get session ID
let session_id = session.id();

// Sessions are automatically expired based on JWT expiration (if using auth)
// No manual expiration API for tools - handled by transport
```

## Sessions and Authentication

When using JWT authentication, sessions can be bound to user identity:

**Transport layer automatically stores:**
- `jwt:sub` - Subject (user ID)
- `jwt:iss` - Issuer
- `jwt:scopes` - Comma-separated scopes
- `jwt:audiences` - Comma-separated audiences
- `jwt:exp` - Expiration timestamp
- `jwt:iat` - Issued-at timestamp

**Session TTL matches JWT expiration** - sessions automatically expire when JWT expires.

**Accessing identity in tools:**
```rust
fn call_tool(ctx: MessageContext, request: CallToolRequest) -> Result<CallToolResult, String> {
    // Get user identity
    let Some(identity) = ctx.identity else {
        return Err("Authentication required".to_string());
    };

    let user_id = get_subject(&identity.claims);

    // Open session
    let session = Session::open(
        &ctx.session.as_ref().unwrap().session_id,
        &ctx.session.as_ref().unwrap().store_id
    )?;

    // Store user-specific data
    session.set("last_user", &TypedValue::AsString(user_id))?;

    Ok(result)
}
```

**See:** [authentication-and-authorization.md](./authentication-and-authorization.md)

## Testing with Sessions

### Local Testing

**Start server with session support:**
```bash
wasmcp compose server my-tool -o server.wasm
wasmtime serve -Scli -Skeyvalue -Shttp server.wasm
```

**Test session creation:**
```bash
# First request - no session ID
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "add_to_cart", "arguments": {"item": "apple"}}}'

# Response includes Mcp-Session-Id header
# Use it in subsequent requests:

curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: <session-id-from-response>" \
  -d '{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "add_to_cart", "arguments": {"item": "banana"}}}'
```

### Mock Sessions in Tests

**Rust (unit tests):**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        // Mock session info
        let session_info = SessionInfo {
            session_id: "test-session".to_string(),
            store_id: "test-store".to_string(),
        };

        // Note: Full session testing requires integration tests
        // with actual kv-store component
    }
}
```

**Integration tests:**
```bash
# Compose test server
wasmcp compose server counter -o test-server.wasm

# Run tests
./test-scripts/test-session-behavior.sh
```

## Error Handling

**Common errors:**

```rust
// Session not provided (sessions disabled)
if ctx.session.is_none() {
    return Err("Sessions not enabled".to_string());
}

// Failed to open session
let session = Session::open(&session_id, &store_id)
    .map_err(|e| match e {
        SessionError::NoSuchSession => "Session expired or invalid".to_string(),
        SessionError::Store(msg) => format!("Storage error: {}", msg),
        _ => format!("Session error: {:?}", e),
    })?;

// Failed to get/set value
session.get("key")
    .map_err(|e| format!("Failed to read session: {:?}", e))?;

session.set("key", &value)
    .map_err(|e| format!("Failed to write session: {:?}", e))?;
```

## Session Configuration

**Enable sessions (default):**
```bash
# HTTP transport automatically enables sessions
wasmcp compose server my-tool -o server.wasm
```

**Disable sessions:**
```bash
# Sessions are always available but clients can choose not to use them
# by not sending Mcp-Session-Id header
```

**Custom session store:**
```bash
# Use different kv-store implementation
wasmcp compose server my-tool --override-kv-store custom-kv.wasm -o server.wasm
```

## Best Practices

### 1. Check for Session Availability

Not all transports or configurations provide sessions:

```rust
let session = match &ctx.session {
    Some(info) => Session::open(&info.session_id, &info.store_id)?,
    None => {
        // Fallback for stateless operation
        return stateless_operation(request);
    }
};
```

### 2. Use Appropriate Types

Choose TypedValue variant based on data:

- **AsU64/AsS64** - Counters, timestamps, IDs
- **AsString** - Names, short text, UUIDs
- **AsJson** - Complex objects, arrays
- **AsBool** - Flags, toggles
- **AsBytes** - Binary data, encrypted values

### 3. Namespace Session Keys

Prevent key collisions:

```rust
// Good: namespace by feature
session.set("cart:items", &value)?;
session.set("user:preferences", &value)?;

// Bad: generic keys
session.set("items", &value)?;
session.set("prefs", &value)?;
```

### 4. Handle Missing Keys Gracefully

```rust
let count = match session.get("counter")? {
    Some(TypedValue::AsU64(n)) => n,
    _ => 0, // Default value
};
```

### 5. Clean Up After Operations

```rust
// Clear temporary data
fn complete_wizard(session: &Session) -> Result<(), String> {
    // Process wizard data
    let data = get_wizard_data(session)?;
    process_data(data)?;

    // Clean up
    session.set("wizard:step", &TypedValue::AsU64(0))?;
    session.set("wizard:data", &TypedValue::AsJson("{}".to_string()))?;

    Ok(())
}
```

### 6. Consider Session Size

Sessions are stored in key-value stores with size limits:

- Keep individual values under 1MB
- Store large blobs externally (S3, etc.), keep references in session
- Periodically clean up old session data

### 7. Security Considerations

- **Don't store sensitive data in plain text** - Encrypt before storing
- **Validate session ownership** - Use identity claims to verify user
- **Set appropriate expiration** - Sessions inherit JWT expiration
- **Clear session on logout** - Call `terminate()` explicitly

## Troubleshooting

### "Session not found"

**Cause:** Session expired, deleted, or invalid ID

**Solution:**
- Client should handle session expiration
- Re-initialize session by making request without session ID
- Check JWT expiration if using auth

### "Failed to open session: Store error"

**Cause:** Key-value store unavailable or misconfigured

**Solution:**
- Ensure `wasmtime serve -Skeyvalue` includes keyvalue capability
- Check kv-store component is properly composed
- Verify WASI keyvalue implementation

### "Type mismatch"

**Cause:** Stored TypedValue doesn't match expected type

**Solution:**
```rust
// Handle multiple possible types
match session.get("count")? {
    Some(TypedValue::AsU64(n)) => n,
    Some(TypedValue::AsString(s)) => s.parse().unwrap_or(0),
    Some(TypedValue::AsS64(n)) => n as u64,
    _ => 0,
}
```

### Session data not persisting

**Cause:** Using stdio transport (sessions work but aren't persisted across process restarts)

**Solution:**
- HTTP transport persists sessions in kv-store
- For persistence, ensure kv-store backend supports durable storage

## Related Documentation

- **[message-context.md](./message-context.md)** - Full MessageContext reference
- **[typed-value.md](./typed-value.md)** - TypedValue API details
- **[authentication-and-authorization.md](./authentication-and-authorization.md)** - Using identity with sessions
- **[examples/counter-middleware](../examples/counter-middleware/)** - Working session example

## Session WIT Interface

Full interface definition: [spec/2025-06-18/wit/sessions.wit](../spec/2025-06-18/wit/sessions.wit)

**Key types:**
```wit
resource session {
    open: static func(session-id: string, store-id: string) -> result<session, session-error>;
    id: func() -> string;
    get: func(key: string) -> result<option<typed-value>, session-error>;
    set: func(key: string, value: typed-value) -> result<_, session-error>;
    terminate: func(reason: option<string>) -> result<_, session-error>;
}

variant session-error {
    io(stream-error),
    store(string),
    no-such-session,
    unexpected(string),
}
```
