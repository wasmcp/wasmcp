# kv-store

A typed key-value storage component that abstracts [wasi:keyvalue](https://github.com/WebAssembly/wasi-keyvalue) with runtime type safety and version compatibility.

## Overview

The `kv-store` component provides:

- **Type-safe storage** - TypedValue enum with runtime type validation
- **Version abstraction** - Supports both `wasi:keyvalue` draft and draft2 via feature flags
- **Environment integration** - Reads `WASMCP_SESSION_BUCKET` for default bucket selection
- **Comprehensive API** - Generic typed operations + convenience methods for each type
- **Batch operations** - Multi-key get/set/delete for performance

## TypedValue System

All stored values include a 1-byte type tag prefix for runtime type safety:

```rust
variant typed-value {
    as-string(string),      // 0x01 + UTF-8 bytes
    as-json(string),        // 0x02 + validated JSON bytes
    as-u64(u64),            // 0x03 + 8-byte little-endian
    as-s64(s64),            // 0x04 + 8-byte little-endian
    as-bool(bool),          // 0x05 + single byte (0/1)
    as-bytes(list<u8>)      // 0x06 + raw bytes
}
```

**Benefits:**
- **Type safety** - Get operations validate the stored type matches expected type
- **Introspection** - Generic `get()` returns type information for tooling
- **JSON validation** - Syntax checking on write for `as-json` values
- **Binary safety** - `as-bytes` supports arbitrary binary data

## Integration Guide

### Adding to Your Component

1. **Import in WIT:**

```wit
// wit/world.wit
package my:component;

world my-world {
    import wasmcp:keyvalue/store@0.1.0;
    export wasmcp:mcp-v20250618/tools@0.1.7;
}
```

2. **Use in code:**

```rust
mod bindings {
    wit_bindgen::generate!({
        world: "my-world",
        generate_all,
    });
}

use bindings::wasmcp::keyvalue::store::{self as kv_store, TypedValue};

fn my_tool_handler() -> Result<()> {
    let bucket = kv_store::open("")?;
    bucket.set_string("my-key", "my-value".to_string())?;
    Ok(())
}
```

## Usage Examples

### Session Storage Pattern

From session-store crate:

```rust
use wasmcp::keyvalue::store::{self as kv_store, TypedValue};

pub fn save_session_data(
    session_id: &str,
    store_id: &str,
    key: &str,
    value: TypedValue
) -> Result<(), Error> {
    let bucket = kv_store::open(store_id)?;

    // Namespace keys by session ID
    let kv_key = format!("{}:{}", session_id, key);
    bucket.set(&kv_key, &value)?;

    Ok(())
}

pub fn load_session_data(
    session_id: &str,
    store_id: &str,
    key: &str
) -> Result<Option<TypedValue>, Error> {
    let bucket = kv_store::open(store_id)?;
    let kv_key = format!("{}:{}", session_id, key);
    bucket.get(&kv_key)
}
```

### Todo List Storage Pattern

From todo-list-auth example:

```rust
use wasmcp::keyvalue::store::TypedValue;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct TodoItem {
    id: usize,
    title: String,
}

fn save_todo_list(session_id: &str, store_id: &str, items: &[TodoItem]) -> Result<()> {
    let session = sessions::Session::open(session_id, store_id)?;

    // Serialize to JSON bytes
    let json_bytes = serde_json::to_vec(items)?;

    // Store as typed binary
    session.set("todo:list", &TypedValue::AsBytes(json_bytes))?;
    Ok(())
}

fn load_todo_list(session_id: &str, store_id: &str) -> Result<Vec<TodoItem>> {
    let session = sessions::Session::open(session_id, store_id)?;

    match session.get("todo:list")? {
        Some(TypedValue::AsBytes(bytes)) => {
            Ok(serde_json::from_slice(&bytes).unwrap_or_default())
        }
        _ => Ok(Vec::new())
    }
}
```

### Counter Pattern

```rust
use wasmcp::keyvalue::store::TypedValue;

fn track_api_calls(bucket: &Bucket, endpoint: &str) -> Result<u64> {
    let key = format!("metrics:calls:{}", endpoint);
    let count = bucket.increment(&key, 1)?;
    Ok(count as u64)
}

fn get_metrics(bucket: &Bucket, endpoint: &str) -> Result<u64> {
    let key = format!("metrics:calls:{}", endpoint);
    match bucket.get_s64(&key)? {
        Some(n) => Ok(n as u64),
        None => Ok(0)
    }
}
```

### Typed Configuration Pattern

```rust
#[derive(Serialize, Deserialize)]
struct AppConfig {
    theme: String,
    notifications: bool,
}

fn save_config(bucket: &Bucket, user_id: &str, config: &AppConfig) -> Result<()> {
    let json = serde_json::to_string(config)?;
    bucket.set_json(&format!("config:{}", user_id), json)?;
    Ok(())
}

fn load_config(bucket: &Bucket, user_id: &str) -> Result<Option<AppConfig>> {
    match bucket.get_json(&format!("config:{}", user_id))? {
        Some(json) => Ok(Some(serde_json::from_str(&json)?)),
        None => Ok(None)
    }
}
```

### Runtime Configuration

**Spin runtime:**

```toml
# spin.toml
[component.my-component]
key_value_stores = ["default"]
environment = { WASMCP_SESSION_BUCKET = "default" }
```

**Wasmtime runtime:**

The `wasmtime` wasi:key-value implementation currently does not maintain state between requests, so it is generally not advised/realistic to use kv-store if using `wasmtime`

### Composition

The kv-store component is automatically included when composing components that import `wasmcp:keyvalue/store`:

```bash
# CLI detects the import and includes kv-store.wasm
wasmcp compose server my-tool.wasm -o server.wasm
```

Explicit override:

```bash
wasmcp compose server my-tool.wasm \
  --override-kv-store custom-kv.wasm \
  -o server.wasm
```

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for development setup and testing guidelines.

## License

Apache 2.0 - See [LICENSE](../../LICENSE) for details.
