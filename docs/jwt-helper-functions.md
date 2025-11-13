# JWT Helper Functions Reference

Quick reference for `wasmcp:oauth/helpers` functions.

## Import

```rust
// Rust
use wasmcp::oauth::helpers::{
    has_scope, has_any_scope, has_all_scopes,
    has_audience, is_expired, is_valid_time,
    get_subject, get_issuer, get_scopes, get_audiences,
    get_claim, flatten_claims,
};
```

```python
# Python
from wasmcp.oauth.helpers import (
    has_scope, has_any_scope, has_all_scopes,
    has_audience, is_expired, is_valid_time,
    get_subject, get_issuer, get_scopes, get_audiences,
    get_claim, flatten_claims
)
```

---

## Scope Validation

### `has-scope(claims, scope) -> bool`

Check if specific scope exists.

```rust
if has_scope(&claims, "read:users") {
    // User has permission to read users
}
```

### `has-any-scope(claims, scopes) -> bool`

Check if **any** of the scopes exist (OR logic).

```rust
if has_any_scope(&claims, &["admin", "moderator"]) {
    // User has either admin OR moderator
}
```

### `has-all-scopes(claims, scopes) -> bool`

Check if **all** scopes exist (AND logic).

```rust
if has_all_scopes(&claims, &["read:data", "write:data"]) {
    // User has both read AND write
}
```

### `get-scopes(claims) -> list<string>`

Get all scopes as list.

```rust
let scopes = get_scopes(&claims);
// ["read:users", "write:users", "admin"]
```

---

## Audience Validation

### `has-audience(claims, audience) -> bool`

**CRITICAL**: Always validate audience to prevent confused deputy attacks!

```rust
if !has_audience(&claims, "https://api.example.com") {
    return Err("Token not intended for this service");
}
```

### `get-audiences(claims) -> list<string>`

Get all audiences as list.

```rust
let audiences = get_audiences(&claims);
// ["https://api.example.com", "https://admin.example.com"]
```

---

## Time Validation

### `is-expired(claims, clock-skew-seconds) -> bool`

Check if token has expired.

```rust
// Check with 60 second clock skew tolerance
if is_expired(&claims, Some(60)) {
    return Err("Token has expired");
}
```

### `is-valid-time(claims, clock-skew-seconds) -> bool`

Check both `nbf` (not before) and `exp` (expiration).

```rust
if !is_valid_time(&claims, Some(60)) {
    return Err("Token not yet valid or has expired");
}
```

---

## Standard Claims

### `get-subject(claims) -> string`

Get user ID (`sub` claim).

```rust
let user_id = get_subject(&claims);
// "user_123abc"
```

### `get-issuer(claims) -> option<string>`

Get token issuer (`iss` claim).

```rust
if let Some(issuer) = get_issuer(&claims) {
    // "https://auth.example.com"
}
```

---

## Custom Claims

### `get-claim(claims, key) -> option<string>`

Get custom claim value by key.

```rust
// Get organization ID from custom claims
if let Some(org_id) = get_claim(&claims, "org_id") {
    // Use org_id for multi-tenant authorization
}

// Get role
if let Some(role) = get_claim(&claims, "role") {
    if role == "admin" {
        // Admin-specific logic
    }
}
```

---

## Utility Functions

### `flatten-claims(claims) -> list<tuple<string, string>>`

Convert structured claims to flat key-value pairs.

Useful for logging or storing in session KV.

```rust
let flat = flatten_claims(&claims);
// [("sub", "user_123"), ("iss", "https://auth.example.com"), ...]
```

---

## Common Patterns

### Pattern: Require Specific Scope

```rust
fn admin_tool(context: MessageContext) -> Result<String, String> {
    let identity = context.identity.ok_or("Auth required")?;

    if !has_scope(&identity.claims, "admin") {
        return Err("Requires admin scope");
    }

    // Admin operation
    Ok("Success".to_string())
}
```

### Pattern: Require One of Multiple Scopes

```rust
fn moderation_tool(context: MessageContext) -> Result<String, String> {
    let identity = context.identity.ok_or("Auth required")?;

    if !has_any_scope(&identity.claims, &["admin", "moderator"]) {
        return Err("Requires admin or moderator scope");
    }

    // Moderation operation
    Ok("Success".to_string())
}
```

### Pattern: Validate Audience + Scopes

```rust
fn secure_api_call(context: MessageContext) -> Result<String, String> {
    let identity = context.identity.ok_or("Auth required")?;

    // CRITICAL: Always validate audience first!
    if !has_audience(&identity.claims, "https://api.example.com") {
        return Err("Token not intended for this service");
    }

    // Then check scopes
    if !has_scope(&identity.claims, "api:read") {
        return Err("Requires api:read scope");
    }

    // Safe to proceed
    Ok("API data".to_string())
}
```

### Pattern: Multi-Tenant Data Access

```rust
fn get_tenant_data(context: MessageContext, tenant_id: String) -> Result<Data, String> {
    let identity = context.identity.ok_or("Auth required")?;

    // Check organization membership
    if let Some(org_id) = get_claim(&identity.claims, "org_id") {
        if org_id != tenant_id {
            return Err("Not authorized for this tenant");
        }
    } else if let Some(org_ids_str) = get_claim(&identity.claims, "org_ids") {
        let org_ids: Vec<&str> = org_ids_str.split(',').collect();
        if !org_ids.contains(&tenant_id.as_str()) {
            return Err("Not authorized for this tenant");
        }
    } else {
        return Err("No organization claims found");
    }

    // User is authorized for this tenant
    Ok(fetch_tenant_data(&tenant_id))
}
```

### Pattern: Log User Actions

```rust
fn sensitive_operation(context: MessageContext) -> Result<(), String> {
    let identity = context.identity.ok_or("Auth required")?;
    let user_id = get_subject(&identity.claims);

    // Log the action
    eprintln!("[AUDIT] User {} attempting sensitive operation", user_id);

    if !has_scope(&identity.claims, "admin") {
        eprintln!("[AUDIT] User {} denied - missing admin scope", user_id);
        return Err("Requires admin scope");
    }

    // Perform operation
    perform_sensitive_action();

    eprintln!("[AUDIT] User {} completed sensitive operation", user_id);
    Ok(())
}
```

---

## Security Checklist

When implementing authorization:

- [ ] ✅ Validate audience with `has-audience()` (prevents confused deputy)
- [ ] ✅ Check required scopes with `has-scope()` or `has-all-scopes()`
- [ ] ✅ Validate expiration with `is-expired()` or `is-valid-time()`
- [ ] ✅ Filter database queries by user/organization
- [ ] ✅ Use least privilege (request only needed scopes)
- [ ] ✅ Log authorization failures for security monitoring
- [ ] ✅ Handle both authenticated and unauthenticated cases

---

## See Also

- [Authentication and Authorization Guide](./authentication-and-authorization.md) - Complete guide with examples
- [JWT Claims WIT Specification](../spec/oauth/wit/types.wit) - Type definitions
- [Helper Functions WIT Specification](../spec/oauth/wit/helpers.wit) - Function signatures
