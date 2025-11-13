# TypedValue API Reference

Complete guide to using `TypedValue` for type-safe session and key-value storage in wasmcp.

## Overview

`TypedValue` is an enum that wraps values with runtime type information. It provides:

- **Type safety** - Stored values include type tags
- **Explicit serialization** - Choose string, JSON, number, bool, or bytes
- **Runtime introspection** - Check type before use
- **Forward compatibility** - Add types without breaking changes

Used by:
- Session storage (`session.get()` / `session.set()`)
- Key-value stores (`bucket.get()` / `bucket.set()`)

## Type Definition

**WIT (spec/keyvalue/wit/store.wit:36-54):**
```wit
variant typed-value {
    /// UTF-8 string value
    as-string(string),

    /// JSON string (validated for JSON syntax on write)
    as-json(string),

    /// Unsigned 64-bit integer
    as-u64(u64),

    /// Signed 64-bit integer
    as-s64(s64),

    /// Boolean value
    as-bool(bool),

    /// Raw binary data
    as-bytes(list<u8>)
}
```

## Variants

### AsString - Plain Text

**Use for:**
- Names, labels, short text
- UUIDs, identifiers
- Enum-like values ("pending", "completed")
- Non-JSON structured data

**Rust:**
```rust
use crate::bindings::wasmcp::keyvalue::store::TypedValue;

// Store
session.set("user_name", &TypedValue::AsString("Alice".to_string()))?;
session.set("status", &TypedValue::AsString("pending".to_string()))?;

// Retrieve
match session.get("user_name")? {
    Some(TypedValue::AsString(name)) => {
        println!("User: {}", name);
    }
    Some(other) => {
        eprintln!("Expected string, got: {:?}", other);
    }
    None => {
        println!("Not found");
    }
}
```

**Python:**
```python
from wasmcp.keyvalue.store import TypedValue

# Store
session.set("user_name", TypedValue.AsString("Alice"))
session.set("status", TypedValue.AsString("pending"))

# Retrieve
value = session.get("user_name")
if value is None:
    print("Not found")
elif isinstance(value, TypedValue.AsString):
    print(f"User: {value.value}")
else:
    print(f"Expected string, got: {type(value)}")
```

---

### AsJson - Structured Data

**Use for:**
- Objects, arrays
- Complex nested structures
- Data that needs schema evolution
- API responses

**Rust:**
```rust
use serde::{Serialize, Deserialize};
use serde_json;

#[derive(Serialize, Deserialize)]
struct Cart {
    items: Vec<String>,
    total: f64,
}

// Store
let cart = Cart {
    items: vec!["apple".to_string(), "banana".to_string()],
    total: 3.50,
};
let json = serde_json::to_string(&cart)?;
session.set("cart", &TypedValue::AsJson(json))?;

// Retrieve
match session.get("cart")? {
    Some(TypedValue::AsJson(json)) => {
        let cart: Cart = serde_json::from_str(&json)?;
        println!("Items: {:?}", cart.items);
    }
    _ => println!("Cart not found"),
}
```

**Python:**
```python
import json

# Store
cart = {
    "items": ["apple", "banana"],
    "total": 3.50
}
session.set("cart", TypedValue.AsJson(json.dumps(cart)))

# Retrieve
value = session.get("cart")
if isinstance(value, TypedValue.AsJson):
    cart = json.loads(value.value)
    print(f"Items: {cart['items']}")
```

**Validation:**
- JSON syntax validated on write
- Invalid JSON returns error immediately
- Valid JSON guaranteed on read

---

### AsU64 - Unsigned Integers

**Use for:**
- Counters (always positive)
- Timestamps (Unix seconds)
- IDs, indices
- Quantities, counts

**Rust:**
```rust
// Store
session.set("counter", &TypedValue::AsU64(42))?;
session.set("timestamp", &TypedValue::AsU64(1700000000))?;

// Retrieve
match session.get("counter")? {
    Some(TypedValue::AsU64(count)) => {
        let new_count = count + 1;
        session.set("counter", &TypedValue::AsU64(new_count))?;
    }
    _ => {
        // Initialize if missing
        session.set("counter", &TypedValue::AsU64(1))?;
    }
}
```

**Python:**
```python
# Store
session.set("counter", TypedValue.AsU64(42))
session.set("timestamp", TypedValue.AsU64(1700000000))

# Retrieve
value = session.get("counter")
if isinstance(value, TypedValue.AsU64):
    new_count = value.value + 1
    session.set("counter", TypedValue.AsU64(new_count))
else:
    # Initialize if missing
    session.set("counter", TypedValue.AsU64(1))
```

**Range:** 0 to 18,446,744,073,709,551,615

---

### AsS64 - Signed Integers

**Use for:**
- Deltas (positive or negative)
- Scores, rankings
- Temperature, offsets
- Financial amounts (cents)

**Rust:**
```rust
// Store
session.set("balance", &TypedValue::AsS64(1000))?;  // $10.00 in cents
session.set("delta", &TypedValue::AsS64(-50))?;

// Retrieve and modify
match session.get("balance")? {
    Some(TypedValue::AsS64(balance)) => {
        let new_balance = balance - 50;  // Subtract 50 cents
        session.set("balance", &TypedValue::AsS64(new_balance))?;
    }
    _ => {}
}
```

**Python:**
```python
# Store
session.set("balance", TypedValue.AsS64(1000))  # $10.00 in cents
session.set("delta", TypedValue.AsS64(-50))

# Retrieve and modify
value = session.get("balance")
if isinstance(value, TypedValue.AsS64):
    new_balance = value.value - 50  # Subtract 50 cents
    session.set("balance", TypedValue.AsS64(new_balance))
```

**Range:** -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807

---

### AsBool - Boolean Flags

**Use for:**
- Feature flags
- Preferences (enabled/disabled)
- State flags (is_premium, has_accepted_terms)
- Binary choices

**Rust:**
```rust
// Store
session.set("premium", &TypedValue::AsBool(true))?;
session.set("notifications_enabled", &TypedValue::AsBool(false))?;

// Retrieve
match session.get("premium")? {
    Some(TypedValue::AsBool(true)) => {
        // Premium features
    }
    Some(TypedValue::AsBool(false)) | None => {
        // Free tier
    }
    _ => {}
}
```

**Python:**
```python
# Store
session.set("premium", TypedValue.AsBool(True))
session.set("notifications_enabled", TypedValue.AsBool(False))

# Retrieve
value = session.get("premium")
if isinstance(value, TypedValue.AsBool) and value.value:
    # Premium features
    pass
else:
    # Free tier
    pass
```

---

### AsBytes - Raw Binary Data

**Use for:**
- Encrypted data
- Compressed data
- Binary protocols
- Images, files (small)

**Rust:**
```rust
// Store
let data = vec![0x01, 0x02, 0x03, 0x04];
session.set("encrypted", &TypedValue::AsBytes(data))?;

// Retrieve
match session.get("encrypted")? {
    Some(TypedValue::AsBytes(bytes)) => {
        // Decrypt or process
        let decrypted = decrypt(&bytes)?;
    }
    _ => {}
}
```

**Python:**
```python
# Store
data = bytes([0x01, 0x02, 0x03, 0x04])
session.set("encrypted", TypedValue.AsBytes(data))

# Retrieve
value = session.get("encrypted")
if isinstance(value, TypedValue.AsBytes):
    # Decrypt or process
    decrypted = decrypt(value.value)
```

**Size limit:** Check storage backend limits (typically MB range)

---

## Storage Format

**Internal representation (implementation detail):**
```
[1 byte: type tag][N bytes: serialized data]
```

**Type tags:**
- `0x01` - AsString (UTF-8 string)
- `0x02` - AsJson (JSON string)
- `0x03` - AsU64 (8 bytes, little-endian)
- `0x04` - AsS64 (8 bytes, little-endian)
- `0x05` - AsBool (1 byte: 0x00 or 0x01)
- `0x06` - AsBytes (raw bytes)

**Benefits:**
- Type checking on retrieval
- Generic tooling can introspect types
- Forward compatible (add new tags)

---

## Common Patterns

### Pattern 1: Type-Safe Counter

```rust
fn increment_counter(session: &Session, key: &str) -> Result<u64, String> {
    let current = match session.get(key)? {
        Some(TypedValue::AsU64(n)) => n,
        Some(TypedValue::AsString(s)) => {
            // Handle legacy string format
            s.parse::<u64>().unwrap_or(0)
        }
        _ => 0,
    };

    let new_count = current + 1;
    session.set(key, &TypedValue::AsU64(new_count))?;
    Ok(new_count)
}
```

### Pattern 2: Fallback for Missing Types

```rust
fn get_string_flexible(session: &Session, key: &str) -> Result<String, String> {
    match session.get(key)? {
        Some(TypedValue::AsString(s)) => Ok(s),
        Some(TypedValue::AsJson(json)) => Ok(json),  // JSON as string
        Some(TypedValue::AsU64(n)) => Ok(n.to_string()),
        Some(TypedValue::AsS64(n)) => Ok(n.to_string()),
        Some(TypedValue::AsBool(b)) => Ok(b.to_string()),
        Some(TypedValue::AsBytes(bytes)) => {
            String::from_utf8(bytes).map_err(|e| e.to_string())
        }
        None => Err("Key not found".to_string()),
    }
}
```

### Pattern 3: Type Migration

```rust
fn get_count_with_migration(session: &Session) -> Result<u64, String> {
    match session.get("count")? {
        // New format
        Some(TypedValue::AsU64(n)) => Ok(n),

        // Old format (string)
        Some(TypedValue::AsString(s)) => {
            let n = s.parse::<u64>().unwrap_or(0);
            // Migrate to new format
            session.set("count", &TypedValue::AsU64(n))?;
            Ok(n)
        }

        _ => Ok(0),
    }
}
```

### Pattern 4: Generic Storage Helper

```rust
fn store_value<T: Serialize>(
    session: &Session,
    key: &str,
    value: &T
) -> Result<(), String> {
    let json = serde_json::to_string(value)
        .map_err(|e| format!("Serialization failed: {}", e))?;
    session.set(key, &TypedValue::AsJson(json))?;
    Ok(())
}

fn load_value<T: DeserializeOwned>(
    session: &Session,
    key: &str
) -> Result<Option<T>, String> {
    match session.get(key)? {
        Some(TypedValue::AsJson(json)) => {
            let value = serde_json::from_str(&json)
                .map_err(|e| format!("Deserialization failed: {}", e))?;
            Ok(Some(value))
        }
        None => Ok(None),
        _ => Err("Type mismatch".to_string()),
    }
}
```

### Pattern 5: Multi-Type Getter

```rust
enum StoredValue {
    Text(String),
    Number(i64),
    Flag(bool),
    Data(Vec<u8>),
}

fn get_any(session: &Session, key: &str) -> Result<Option<StoredValue>, String> {
    match session.get(key)? {
        Some(TypedValue::AsString(s)) => Ok(Some(StoredValue::Text(s))),
        Some(TypedValue::AsU64(n)) => Ok(Some(StoredValue::Number(n as i64))),
        Some(TypedValue::AsS64(n)) => Ok(Some(StoredValue::Number(n))),
        Some(TypedValue::AsBool(b)) => Ok(Some(StoredValue::Flag(b))),
        Some(TypedValue::AsBytes(data)) => Ok(Some(StoredValue::Data(data))),
        Some(TypedValue::AsJson(json)) => Ok(Some(StoredValue::Text(json))),
        None => Ok(None),
    }
}
```

## Choosing the Right Type

| Data | Recommended Type | Alternative |
|------|-----------------|-------------|
| User names, labels | `AsString` | - |
| Counters (0+) | `AsU64` | `AsString` (legacy) |
| Deltas, scores | `AsS64` | `AsString` |
| Feature flags | `AsBool` | `AsString("true"/"false")` |
| Timestamps (Unix) | `AsU64` | `AsString` |
| Objects, arrays | `AsJson` | - |
| Encrypted data | `AsBytes` | - |
| UUIDs, IDs | `AsString` | - |
| Currency (cents) | `AsS64` | `AsString` |
| Enums | `AsString` | `AsJson` |

## Error Handling

**Common errors:**

```rust
// Key not found (not an error)
match session.get("key")? {
    Some(value) => { /* use value */ }
    None => { /* key doesn't exist */ }
}

// Type mismatch
match session.get("counter")? {
    Some(TypedValue::AsU64(n)) => { /* expected type */ }
    Some(other) => {
        return Err(format!("Expected u64, got {:?}", other));
    }
    None => { /* handle missing */ }
}

// Storage error
session.set("key", &value)
    .map_err(|e| format!("Failed to save: {:?}", e))?;

// JSON validation error (on write)
let invalid_json = "{ not valid }";
session.set("data", &TypedValue::AsJson(invalid_json.to_string()))
    .expect_err("Should fail JSON validation");
```

## Performance Considerations

**Type overhead:**
- 1 byte per value (type tag)
- Negligible for most use cases

**Serialization cost:**
- `AsString`, `AsJson`: UTF-8 encoding
- `AsU64`, `AsS64`: 8 bytes (fixed)
- `AsBool`: 1 byte
- `AsBytes`: No overhead

**Best practices:**
- Use numeric types (`AsU64`/`AsS64`) for counters (faster than parsing strings)
- Use `AsJson` for complex structures (easier than manual serialization)
- Avoid `AsBytes` for small strings (use `AsString`)
- Batch operations when possible (see bucket.set-many)

## Migration from Raw Bytes

**Before (old API):**
```rust
// Storing
let bytes = "hello".as_bytes();
session.set("key", bytes)?;

// Reading
let bytes: Vec<u8> = session.get("key")?;
let text = String::from_utf8(bytes)?;
```

**After (TypedValue API):**
```rust
// Storing
session.set("key", &TypedValue::AsString("hello".to_string()))?;

// Reading
match session.get("key")? {
    Some(TypedValue::AsString(text)) => { /* use text */ }
    _ => { /* handle type mismatch */ }
}
```

**Migration helper:**
```rust
fn migrate_to_typed_value(session: &Session) -> Result<(), String> {
    // Read keys to migrate
    let keys = vec!["counter", "name", "enabled"];

    for key in keys {
        match session.get(key)? {
            // Already migrated
            Some(TypedValue::AsString(_)) |
            Some(TypedValue::AsU64(_)) |
            Some(TypedValue::AsBool(_)) => continue,

            // Legacy bytes format
            Some(TypedValue::AsBytes(bytes)) => {
                // Determine type and migrate
                if key == "counter" {
                    let s = String::from_utf8(bytes)?;
                    let n = s.parse::<u64>()?;
                    session.set(key, &TypedValue::AsU64(n))?;
                } else {
                    let s = String::from_utf8(bytes)?;
                    session.set(key, &TypedValue::AsString(s))?;
                }
            }

            None => {}
        }
    }

    Ok(())
}
```

## Testing

**Mock values in tests:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_increment() {
        // Create mock values
        let zero = TypedValue::AsU64(0);
        let one = TypedValue::AsU64(1);

        // Test logic
        assert_eq!(increment(&zero), one);
    }

    #[test]
    fn test_type_mismatch() {
        let value = TypedValue::AsString("not a number".to_string());

        // Should handle gracefully
        match value {
            TypedValue::AsU64(_) => panic!("Should not be u64"),
            _ => { /* expected */ }
        }
    }
}
```

## Related Documentation

- **[sessions.md](./sessions.md)** - Using TypedValue in sessions
- **[store.wit](../spec/keyvalue/wit/store.wit)** - Full WIT interface
- **[message-context.md](./message-context.md)** - Session field in MessageContext

## WIT Interface

Full interface: [spec/keyvalue/wit/store.wit](../spec/keyvalue/wit/store.wit:36-54)

```wit
variant typed-value {
    as-string(string),
    as-json(string),
    as-u64(u64),
    as-s64(s64),
    as-bool(bool),
    as-bytes(list<u8>)
}
```
