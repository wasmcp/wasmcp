# Authentication and Authorization in wasmcp

This guide explains how to use authentication and authorization in your wasmcp MCP tools.

## Table of Contents

1. [Overview](#overview)
2. [Understanding Identity](#understanding-identity)
3. [Accessing Identity in Tools](#accessing-identity-in-tools)
4. [Common Authorization Patterns](#common-authorization-patterns)
5. [Multi-Tenant Applications](#multi-tenant-applications)
6. [Testing with Mock Identity](#testing-with-mock-identity)
7. [Security Best Practices](#security-best-practices)

---

## Overview

wasmcp provides built-in authentication and authorization through **JWT (JSON Web Token) validation**. When a client sends a request with an `Authorization: Bearer <token>` header, wasmcp:

1. **Validates the JWT** - Checks signature, expiration, issuer, audience
2. **Extracts claims** - Parses token into structured `jwt-claims`
3. **Passes identity** - Makes claims available to your tools via `MessageContext`

Your tools receive validated identity information automatically - no parsing or validation needed!

---

## Understanding Identity

### Identity Structure

When authentication is enabled, tools receive `MessageContext` with an `identity` field:

```rust
// Rust example
use wasmcp::mcp_v20250618::mcp::{MessageContext, Identity};

fn my_tool(context: MessageContext) {
    if let Some(identity) = &context.identity {
        // JWT token (raw bytes)
        let jwt: &[u8] = &identity.jwt;

        // Parsed claims (structured)
        let claims: &JwtClaims = &identity.claims;

        // Access user ID
        let user_id: &str = &claims.subject;

        // Access scopes
        let scopes: &Vec<String> = &claims.scopes;
    } else {
        // No authentication configured or token not provided
    }
}
```

```python
# Python example
from wasmcp import MessageContext, Identity

def my_tool(context: MessageContext):
    if context.identity:
        # Access user ID
        user_id = context.identity.claims.subject

        # Access scopes
        scopes = context.identity.claims.scopes
```

### JWT Claims Fields

The `jwt-claims` structure contains:

**Standard Claims** (RFC 7519):
- `subject` (string) - User ID or subject identifier (REQUIRED)
- `issuer` (option\<string>) - Who issued the token
- `audience` (list\<string>) - Who the token is for
- `expiration` (option\<u64>) - Unix timestamp when token expires
- `issued-at` (option\<u64>) - Unix timestamp when token was issued
- `not-before` (option\<u64>) - Unix timestamp before which token is invalid
- `jwt-id` (option\<string>) - Unique token identifier

**OAuth/Authorization Claims**:
- `scopes` (list\<string>) - OAuth scopes granted to this token
- `confirmation` (option) - Token binding for sender-constrained tokens

**Custom Claims**:
- `custom-claims` (list\<tuple\<string, string>>) - Provider-specific claims

---

## Accessing Identity in Tools

### Checking if User is Authenticated

```rust
// Rust
use wasmcp::mcp_v20250618::mcp::MessageContext;

fn my_tool(context: MessageContext) -> Result<String, String> {
    match &context.identity {
        Some(identity) => {
            let user_id = &identity.claims.subject;
            Ok(format!("Hello, user {}!", user_id))
        }
        None => {
            Err("Authentication required".to_string())
        }
    }
}
```

```python
# Python
def my_tool(context):
    if not context.identity:
        raise Exception("Authentication required")

    user_id = context.identity.claims.subject
    return f"Hello, user {user_id}!"
```

### Using Helper Functions

wasmcp provides helper functions for common authorization checks. Import them from `wasmcp:oauth/helpers` or `wasmcp:mcp-v20250618/sessions`:

```rust
// Rust
use wasmcp::oauth::helpers::{has_scope, has_audience, get_subject};

fn admin_tool(context: MessageContext) -> Result<String, String> {
    let identity = context.identity.ok_or("Authentication required")?;

    // Check if user has admin scope
    if !has_scope(&identity.claims, "admin") {
        return Err("Requires admin scope".to_string());
    }

    // Validate audience
    if !has_audience(&identity.claims, "https://api.example.com") {
        return Err("Token not intended for this service".to_string());
    }

    let user_id = get_subject(&identity.claims);
    Ok(format!("Admin action by {}", user_id))
}
```

**Available Helpers**:

| Function | Description |
|----------|-------------|
| `has-scope(claims, scope)` | Check if single scope exists |
| `has-any-scope(claims, scopes)` | Check if any of the scopes exist |
| `has-all-scopes(claims, scopes)` | Check if all scopes exist |
| `has-audience(claims, aud)` | Validate audience claim |
| `is-expired(claims, skew)` | Check if token is expired |
| `is-valid-time(claims, skew)` | Check nbf and exp claims |
| `get-subject(claims)` | Get user ID |
| `get-issuer(claims)` | Get token issuer |
| `get-scopes(claims)` | Get all scopes |
| `get-audiences(claims)` | Get all audiences |
| `get-claim(claims, key)` | Get custom claim value |

---

## Common Authorization Patterns

### Pattern 1: Scope-Based Authorization

Check if user has specific permissions before allowing operations:

```rust
use wasmcp::oauth::helpers::{has_scope, has_all_scopes};

fn read_data(context: MessageContext) -> Result<Data, String> {
    let identity = context.identity.ok_or("Authentication required")?;

    // Requires "read" scope
    if !has_scope(&identity.claims, "read") {
        return Err("Requires 'read' scope".to_string());
    }

    // Fetch and return data
    Ok(fetch_data())
}

fn write_data(context: MessageContext, data: Data) -> Result<(), String> {
    let identity = context.identity.ok_or("Authentication required")?;

    // Requires both "read" and "write" scopes
    if !has_all_scopes(&identity.claims, &["read", "write"]) {
        return Err("Requires 'read' and 'write' scopes".to_string());
    }

    // Save data
    save_data(data);
    Ok(())
}
```

### Pattern 2: Role-Based Authorization

Use custom claims to check user roles:

```rust
use wasmcp::oauth::helpers::get_claim;

fn admin_operation(context: MessageContext) -> Result<String, String> {
    let identity = context.identity.ok_or("Authentication required")?;

    // Check role from custom claims
    match get_claim(&identity.claims, "role") {
        Some(role) if role == "admin" || role == "superadmin" => {
            // Perform admin operation
            Ok("Admin action completed".to_string())
        }
        Some(role) => {
            Err(format!("Requires admin role, got: {}", role))
        }
        None => {
            Err("No role claim found".to_string())
        }
    }
}
```

### Pattern 3: User-Scoped Data Access

Ensure users can only access their own data:

```rust
use wasmcp::oauth::helpers::get_subject;

fn get_user_profile(context: MessageContext, requested_user_id: String) -> Result<Profile, String> {
    let identity = context.identity.ok_or("Authentication required")?;
    let current_user_id = get_subject(&identity.claims);

    // Users can only access their own profile
    if current_user_id != requested_user_id {
        return Err("Cannot access other users' profiles".to_string());
    }

    Ok(fetch_profile(&requested_user_id))
}
```

### Pattern 4: Audience Validation

**CRITICAL**: Always validate audience to prevent confused deputy attacks:

```rust
use wasmcp::oauth::helpers::has_audience;

fn sensitive_operation(context: MessageContext) -> Result<(), String> {
    let identity = context.identity.ok_or("Authentication required")?;

    // Verify token is intended for this service
    if !has_audience(&identity.claims, "https://myapi.example.com") {
        return Err("Token not intended for this service".to_string());
    }

    // Token is valid for this service, proceed
    Ok(())
}
```

**Why this matters**: Without audience validation, a token intended for ServiceA could be used to access ServiceB if both trust the same issuer. This is called a "confused deputy attack."

---

## Multi-Tenant Applications

### Organization-Scoped Data Access

If your auth provider includes organization claims (common in WorkOS, Auth0, Okta):

```rust
use wasmcp::oauth::helpers::get_claim;

fn get_organization_data(context: MessageContext, org_id: String) -> Result<OrgData, String> {
    let identity = context.identity.ok_or("Authentication required")?;

    // Check org_id claim (for M2M tokens)
    if let Some(token_org_id) = get_claim(&identity.claims, "org_id") {
        if token_org_id != org_id {
            return Err("Not authorized for this organization".to_string());
        }
        return Ok(fetch_org_data(&org_id));
    }

    // Check org_ids claim (for user tokens with multiple orgs)
    if let Some(org_ids_str) = get_claim(&identity.claims, "org_ids") {
        let org_ids: Vec<&str> = org_ids_str.split(',').collect();
        if org_ids.contains(&org_id.as_str()) {
            return Ok(fetch_org_data(&org_id));
        }
    }

    Err("Not a member of this organization".to_string())
}
```

### Tenant Isolation

Ensure data queries are scoped to the user's organization:

```rust
fn list_documents(context: MessageContext) -> Result<Vec<Document>, String> {
    let identity = context.identity.ok_or("Authentication required")?;

    // Extract organization ID
    let org_id = get_claim(&identity.claims, "org_id")
        .ok_or("Missing organization claim")?;

    // Query ONLY returns documents for this organization
    let docs = db_query("SELECT * FROM documents WHERE org_id = ?", &[org_id])?;
    Ok(docs)
}
```

**CRITICAL**: Always filter database queries by organization ID to prevent data leakage between tenants!

---

## Testing with Mock Identity

### Local Development Without Auth

When developing locally, you can disable authentication:

```bash
# Don't set JWT environment variables
# wasmcp will allow unauthenticated requests
```

Your tools should handle both cases:

```rust
fn my_tool(context: MessageContext) -> Result<String, String> {
    match &context.identity {
        Some(identity) => {
            // Production: authenticated user
            let user_id = &identity.claims.subject;
            Ok(format!("Hello, {}!", user_id))
        }
        None => {
            // Development: mock user
            Ok("Hello, test user!".to_string())
        }
    }
}
```

### Testing with Specific Identity

To test authorization logic, you can create mock `Identity` structs:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn mock_identity(subject: &str, scopes: Vec<String>) -> Identity {
        Identity {
            jwt: vec![], // Empty for testing
            claims: JwtClaims {
                subject: subject.to_string(),
                issuer: Some("https://test.example.com".to_string()),
                audience: vec!["https://api.example.com".to_string()],
                expiration: None,
                issued_at: None,
                not_before: None,
                jwt_id: None,
                scopes,
                confirmation: None,
                custom_claims: vec![],
            }
        }
    }

    #[test]
    fn test_admin_tool_with_admin_scope() {
        let context = MessageContext {
            identity: Some(mock_identity("user_123", vec!["admin".to_string()])),
            session_id: None,
        };

        let result = admin_tool(context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_admin_tool_without_admin_scope() {
        let context = MessageContext {
            identity: Some(mock_identity("user_123", vec!["read".to_string()])),
            session_id: None,
        };

        let result = admin_tool(context);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Requires admin scope");
    }
}
```

---

## Security Best Practices

### 1. Always Validate Audience

```rust
// ✅ GOOD: Validates audience
if !has_audience(&claims, "https://myapi.example.com") {
    return Err("Invalid audience");
}

// ❌ BAD: No audience validation (confused deputy attack!)
// Just checking scopes is not enough!
```

### 2. Use Least Privilege Scopes

Request only the scopes your tool actually needs:

```rust
// ✅ GOOD: Specific scopes
if !has_scope(&claims, "read:documents") {
    return Err("Requires read:documents scope");
}

// ❌ BAD: Overly broad scopes
if !has_scope(&claims, "admin") {
    // Don't require admin for non-admin operations!
}
```

### 3. Don't Trust Custom Claims Without Verification

Custom claims can be added by anyone who controls the token:

```rust
// ❌ BAD: Blindly trusting role claim
let role = get_claim(&claims, "role").unwrap_or("user");
if role == "admin" {
    // Dangerous! What if token was issued by untrusted issuer?
}

// ✅ GOOD: Verify issuer first
if get_issuer(&claims) == Some("https://trusted.example.com") {
    let role = get_claim(&claims, "role").unwrap_or("user");
    if role == "admin" {
        // Safe - issuer is trusted
    }
}
```

### 4. Filter Database Queries by User/Org

Never return unfiltered data:

```rust
// ❌ BAD: Returns all users
let users = db_query("SELECT * FROM users")?;

// ✅ GOOD: Filtered by organization
let org_id = get_claim(&claims, "org_id").ok_or("Missing org")?;
let users = db_query("SELECT * FROM users WHERE org_id = ?", &[org_id])?;
```

### 5. Check Token Expiration

wasmcp validates expiration during JWT decoding, but for long-running operations:

```rust
use wasmcp::oauth::helpers::is_expired;

fn long_running_operation(context: MessageContext) -> Result<(), String> {
    let identity = context.identity.ok_or("Auth required")?;

    // Check expiration before starting
    if is_expired(&identity.claims, Some(60)) {
        return Err("Token has expired");
    }

    // ... perform operation ...

    // Check again if operation took a while
    if is_expired(&identity.claims, Some(60)) {
        return Err("Token expired during operation");
    }

    Ok(())
}
```

### 6. Log Authorization Failures

Help with debugging and security monitoring:

```rust
fn admin_tool(context: MessageContext) -> Result<String, String> {
    let identity = context.identity.ok_or("Auth required")?;
    let user_id = get_subject(&identity.claims);

    if !has_scope(&identity.claims, "admin") {
        eprintln!("[AUTH] User {} attempted admin action without admin scope", user_id);
        return Err("Requires admin scope");
    }

    eprintln!("[AUTH] User {} performed admin action", user_id);
    Ok("Success".to_string())
}
```

---

## Summary

**Key Takeaways**:

1. ✅ Identity is automatically validated and passed to tools via `MessageContext`
2. ✅ Use helper functions (`has-scope`, `has-audience`, etc.) for authorization checks
3. ✅ **ALWAYS validate audience** to prevent confused deputy attacks
4. ✅ Filter database queries by user/organization to enforce tenant isolation
5. ✅ Use least privilege - request only needed scopes
6. ✅ Log authorization failures for security monitoring
7. ✅ Handle both authenticated and unauthenticated cases for local development

**Next Steps**:
- See `examples/` for complete tool implementations with authentication
- Read `docs/configuration.md` for JWT validation configuration
- Check `spec/oauth/wit/helpers.wit` for complete helper function reference
