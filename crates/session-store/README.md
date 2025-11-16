# Session Store Component

WebAssembly component that implements stateful session management for wasmcp MCP servers.

## Overview

The session-store component provides two main interfaces:
- **Session Manager** - Creates, validates, and deletes sessions (used by transport layer)
- **Session Resource** - Read/write session data (used by tools/middleware)

## Storage Architecture

### Key Format

All session data uses a namespaced key-value pattern:

```
{session_id}:{user_key}
```

**Examples:**
```
550e8400-e29b-41d4-a716-446655440000:cart:items
550e8400-e29b-41d4-a716-446655440000:user:preferences
550e8400-e29b-41d4-a716-446655440000:__meta__
```

### Session ID Validation

Session IDs must be valid UUID v4 format (enforced by `validate_session_id()`):
- Length: exactly 36 characters
- Format: `8-4-4-4-12` hex digits with hyphens
- Character set: `[0-9a-fA-F-]` only
- Example: `550e8400-e29b-41d4-a716-446655440000`

**Security:** UUID format ensures session IDs never contain colons, maintaining isolation.

### Key Validation

User-provided keys are validated by `validate_user_key()`:

**Allowed:**
- Any UTF-8 string up to 1KB
- Colons for hierarchical naming (e.g., `"cart:items"`, `"user:settings:theme"`)
- Case-sensitive names

**Rejected:**
- Empty strings
- Reserved names: `__meta__`, `__metadata__`, `metadata`, `meta`
- Keys exceeding 1KB

### Isolation Mechanism

Sessions are isolated by prefix matching:

1. **Write:** Keys stored as `format!("{}:{}", session_id, user_key)`
2. **Read:** Keys retrieved using exact `session_id:key` match
3. **Delete:** All keys with prefix `format!("{}:", session_id)` removed
4. **List:** Keys filtered by `starts_with(&session_prefix)`

**Example:**
```rust
// Session A: 550e8400-e29b-41d4-a716-446655440000
session_a.set("count", 1)  // Stores: "550e8400-...:count"

// Session B: 661f9511-f39c-52e5-b827-557766551111
session_b.set("count", 5)  // Stores: "661f9511-...:count"

// No collision - different prefixes
```

## TypedValue Storage

Values are stored using the `TypedValue` enum for type safety:

```rust
pub enum TypedValue {
    AsString(String),      // Text data
    AsJson(String),        // JSON (validated UTF-8)
    AsU64(u64),           // Unsigned integers
    AsS64(i64),           // Signed integers
    AsBool(bool),         // Boolean flags
    AsBytes(Vec<u8>),     // Binary data
}
```

Maximum value size: 10MB

## Session Metadata

Each session has metadata stored at `{session_id}:__meta__`:

```rust
struct SessionMetadata {
    created_at: u64,           // Unix timestamp (seconds)
    last_accessed: u64,        // Unix timestamp (seconds)
    expires_at: Option<u64>,   // Optional expiration
    terminated: bool,          // Soft delete flag
    terminated_reason: Option<String>,
}
```

## Session Lifecycle

### 1. Creation
```rust
SessionManager::create(session_id, store_id)?;
// Creates metadata at: {session_id}:__meta__
```

### 2. Validation
```rust
SessionManager::validate(session_id, store_id)?;
// Checks:
// - Metadata exists
// - Not terminated
// - Not expired
```

### 3. Usage
```rust
let session = Session::open(session_id, store_id)?;
session.set("key", &TypedValue::AsString("value".into()))?;
let value = session.get("key")?;
```

### 4. Termination
```rust
// Soft delete (marks terminated, keeps data)
SessionManager::mark_terminated(session_id, store_id, reason)?;

// Hard delete (removes all data)
SessionManager::delete_session(session_id, store_id)?;
```

## Implementation Notes

### Pagination for Delete

`delete_session()` uses paginated key listing to handle sessions with many keys:

```rust
let mut cursor: Option<String> = None;
loop {
    let response = bucket.list_keys(cursor.as_deref())?;
    let session_keys: Vec<_> = response.keys
        .into_iter()
        .filter(|k| k.starts_with(&session_prefix))
        .collect();

    if !session_keys.is_empty() {
        bucket.delete_many(&session_keys)?;
    }

    cursor = response.cursor;
    if cursor.is_none() { break; }
}
```

### TOCTOU Race Condition

Session validation has a theoretical time-of-check/time-of-use race:
- Session validated as active
- Microseconds pass
- Session could expire or be terminated
- First use occurs

**Mitigation:** Negligible in practice due to WASM per-request model (microseconds between validation and use). Cannot be fixed without atomic compare-and-swap operations in KV store.

### OAuth Helper Functions

The component also contains OAuth/JWT helper functions (in `oauth_helpers.rs`) that were moved here from their original location. These are exported via the `wasmcp:oauth/helpers` interface and consumed by other components like transport.

## Building

```bash
cargo build --release --target wasm32-wasip2
```

Output: `../../target/wasm32-wasip2/release/session_store.wasm`

## WIT Interfaces

### Exports
- `wasmcp:mcp-v20250618/sessions@0.1.6` - Session resource API
- `wasmcp:mcp-v20250618/session-manager@0.1.6` - Lifecycle management

### Imports
- `wasmcp:keyvalue/store@0.1.0` - Underlying KV storage

## Testing

Run tests:
```bash
cargo test
```

Note: Tests require a working KV store implementation or mocks.

## See Also

- User-facing guide: `/docs/sessions.md`
- WIT definitions: `/spec/2025-06-18/wit/sessions.wit`
- KV store interface: `/spec/keyvalue/store.wit`
