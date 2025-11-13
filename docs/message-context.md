# MessageContext Reference

Complete reference for the `MessageContext` type passed to all MCP message handlers.

## Overview

`MessageContext` provides request-scoped information to tools, resources, prompts, and middleware. It includes:

- Output stream for sending notifications to client
- Protocol version
- Session information (if sessions enabled)
- User identity (if authentication enabled)
- Message framing details
- HTTP context (for HTTP transports)

## Type Definition

**WIT (spec/2025-06-18/wit/server.wit:27-41):**
```wit
record message-context {
    /// Output stream for sending messages to the client
    client-stream: option<borrow<output-stream>>,
    /// MCP protocol version
    protocol-version: string,
    /// Client session details, if available
    session: option<session>,
    /// User identity details, if available
    identity: option<identity>,
    /// Message framing for this transport
    frame: message-frame,
    /// HTTP request context (for HTTP transports only)
    http-context: option<http-context>,
}
```

## Fields

### `client_stream: Option<&OutputStream>`

Output stream for sending real-time notifications to the client.

**Use cases:**
- Progress notifications during long-running operations
- Log messages
- Resource update notifications

**Rust example:**
```rust
use crate::bindings::wasmcp::mcp_v20250618::server_io;
use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    ServerMessage, ServerNotification, LoggingMessageNotification, LogLevel
};

fn send_progress(ctx: &MessageContext, message: String) {
    if let Some(stream) = &ctx.client_stream {
        let notification = ServerNotification::Log(LoggingMessageNotification {
            data: message,
            level: LogLevel::Info,
            logger: Some("my-tool".to_string()),
        });
        let msg = ServerMessage::Notification(notification);
        let _ = server_io::send_message(stream, msg, &ctx.frame);
    }
}
```

**Python example:**
```python
from wasmcp.mcp_v20250618 import server_io
from wasmcp.mcp_v20250618.mcp import (
    ServerMessage, ServerNotification, LoggingMessageNotification, LogLevel
)

def send_progress(ctx, message):
    if ctx.client_stream:
        notification = ServerNotification.Log(
            LoggingMessageNotification(
                data=message,
                level=LogLevel.Info,
                logger="my-tool"
            )
        )
        msg = ServerMessage.Notification(notification)
        server_io.send_message(ctx.client_stream, msg, ctx.frame)
```

**Note:** `client_stream` is `None` in some contexts (e.g., batch operations, testing).

---

### `protocol_version: String`

MCP protocol version string (e.g., `"2025-06-18"`).

**Use cases:**
- Feature detection
- Compatibility checks
- Version-specific behavior

**Example:**
```rust
fn handle_request(ctx: &MessageContext) -> Result<Response, String> {
    if ctx.protocol_version != "2025-06-18" {
        return Err(format!("Unsupported protocol version: {}", ctx.protocol_version));
    }
    // ... handle request
}
```

**Note:** wasmcp ensures protocol version compatibility at composition time.

---

### `session: Option<Session>`

Session information if sessions are enabled and session ID provided.

**Type:**
```wit
record session {
    session-id: string,    // UUID v4 formatted session identifier
    store-id: string,      // WASI KV bucket name
}
```

**Use cases:**
- Stateful multi-request workflows
- User preferences
- Shopping carts
- Caching

**Rust example:**
```rust
use crate::bindings::wasmcp::mcp_v20250618::sessions::Session;

fn call_tool(ctx: MessageContext, request: CallToolRequest)
    -> Result<CallToolResult, String>
{
    // Check if session available
    let Some(session_info) = ctx.session else {
        return Err("Session required for this tool".to_string());
    };

    // Open session
    let session = Session::open(&session_info.session_id, &session_info.store_id)
        .map_err(|e| format!("Failed to open session: {:?}", e))?;

    // Use session...
    let counter = increment_counter(&session)?;

    Ok(result)
}
```

**Python example:**
```python
from wasmcp.mcp_v20250618.sessions import Session

def call_tool(ctx, request):
    if not ctx.session:
        raise Exception("Session required for this tool")

    # Open session
    session = Session.open(ctx.session.session_id, ctx.session.store_id)

    # Use session...
    counter = increment_counter(session)

    return result
```

**When `None`:**
- Sessions disabled in transport configuration
- Client didn't provide `Mcp-Session-Id` header
- Session expired or invalid

**See:** [sessions.md](./sessions.md) for complete session guide

---

### `identity: Option<Identity>`

User identity information if authentication is enabled and valid JWT provided.

**Type:**
```wit
record identity {
    claims: jwt-claims,    // Validated JWT claims
}

record jwt-claims {
    subject: string,                    // User ID (sub)
    issuer: option<string>,             // Token issuer (iss)
    audience: list<string>,             // Intended audiences (aud)
    expiration: option<u64>,            // Expiration timestamp (exp)
    not-before: option<u64>,            // Not valid before (nbf)
    issued-at: option<u64>,             // Issued at timestamp (iat)
    jti: option<string>,                // JWT ID
    scopes: list<string>,               // OAuth 2.0 scopes
    custom-claims: list<tuple<string, string>>,  // Additional claims
}
```

**Use cases:**
- Authorization (scope checking)
- User-scoped operations
- Multi-tenant data access
- Audit logging

**Rust example:**
```rust
use crate::bindings::wasmcp::oauth::helpers;

fn call_tool(ctx: MessageContext, request: CallToolRequest)
    -> Result<CallToolResult, String>
{
    // Check authentication
    let Some(identity) = ctx.identity else {
        return Err("Authentication required".to_string());
    };

    // Check authorization
    if !helpers::has_scope(&identity.claims, "admin") {
        return Err("Requires admin scope".to_string());
    }

    // Get user ID
    let user_id = helpers::get_subject(&identity.claims);

    // Perform authorized operation...
    Ok(result)
}
```

**Python example:**
```python
from wasmcp.oauth import helpers

def call_tool(ctx, request):
    if not ctx.identity:
        raise Exception("Authentication required")

    # Check authorization
    if not helpers.has_scope(ctx.identity.claims, "admin"):
        raise Exception("Requires admin scope")

    # Get user ID
    user_id = helpers.get_subject(ctx.identity.claims)

    # Perform authorized operation...
    return result
```

**When `None`:**
- Authentication disabled
- No `Authorization: Bearer <token>` header provided
- JWT validation failed
- Token expired

**See:** [authentication-and-authorization.md](./authentication-and-authorization.md) for complete auth guide

---

### `frame: MessageFrame`

Message framing configuration for transport-specific serialization.

**Type:**
```wit
record message-frame {
    prefix: list<u8>,      // Bytes to prepend to messages
    suffix: list<u8>,      // Bytes to append to messages
}
```

**Use cases:**
- Sending notifications via `server_io::send_message`
- Custom middleware that forwards messages

**Example:**
```rust
use crate::bindings::wasmcp::mcp_v20250618::server_io;

fn send_notification(ctx: &MessageContext, notification: ServerNotification) {
    if let Some(stream) = &ctx.client_stream {
        let msg = ServerMessage::Notification(notification);
        // Frame is applied automatically
        let _ = server_io::send_message(stream, msg, &ctx.frame);
    }
}
```

**Common frame configurations:**

| Transport | Prefix | Suffix | Notes |
|-----------|--------|--------|-------|
| HTTP | `[]` | `[]` | No framing needed |
| Stdio | `[]` | `['\n']` | Newline-delimited JSON |
| Custom | varies | varies | Implementation-specific |

**Note:** Tools typically don't need to inspect frame details directly - use `server_io::send_message` which handles framing automatically.

---

### `http_context: Option<HttpContext>`

HTTP-specific request details (only available in HTTP transport).

**Type:**
```wit
record http-context {
    method: string,              // HTTP method (GET, POST, etc.)
    uri: string,                 // Request URI
    headers: list<tuple<string, list<u8>>>,  // HTTP headers
}
```

**Use cases:**
- Custom authorization logic based on HTTP headers
- Rate limiting by IP
- Tenant identification from custom headers
- Webhook signature verification

**Rust example:**
```rust
fn call_tool(ctx: MessageContext, request: CallToolRequest)
    -> Result<CallToolResult, String>
{
    if let Some(http_ctx) = &ctx.http_context {
        // Check custom header
        for (name, value) in &http_ctx.headers {
            if name.eq_ignore_ascii_case("x-tenant-id") {
                let tenant_id = String::from_utf8_lossy(value);
                // Use tenant_id for scoping...
            }
        }

        // Log request details
        eprintln!("[audit] {} {} from user", http_ctx.method, http_ctx.uri);
    }

    Ok(result)
}
```

**Python example:**
```python
def call_tool(ctx, request):
    if ctx.http_context:
        # Check custom header
        for name, value in ctx.http_context.headers:
            if name.lower() == "x-tenant-id":
                tenant_id = value.decode('utf-8')
                # Use tenant_id for scoping...

        # Log request details
        print(f"[audit] {ctx.http_context.method} {ctx.http_context.uri}")

    return result
```

**When `None`:**
- Using stdio transport
- Using custom non-HTTP transport
- Testing environment

**Security note:** Always validate and sanitize header values before use.

---

## Complete Example

**Rust (comprehensive context usage):**
```rust
use crate::bindings::exports::wasmcp::mcp_v20250618::tools::{
    Guest, CallToolRequest, CallToolResult, MessageContext
};
use crate::bindings::wasmcp::mcp_v20250618::sessions::Session;
use crate::bindings::wasmcp::oauth::helpers;
use crate::bindings::wasmcp::keyvalue::store::TypedValue;

fn call_tool(ctx: MessageContext, request: CallToolRequest)
    -> Result<CallToolResult, String>
{
    // 1. Check protocol version
    if ctx.protocol_version != "2025-06-18" {
        return Err("Unsupported protocol version".to_string());
    }

    // 2. Require authentication
    let Some(identity) = ctx.identity else {
        return Err("Authentication required".to_string());
    };

    // 3. Check authorization
    if !helpers::has_scope(&identity.claims, "api:read") {
        return Err("Missing required scope: api:read".to_string());
    }

    // 4. Get user ID
    let user_id = helpers::get_subject(&identity.claims);

    // 5. Open session
    let Some(session_info) = ctx.session else {
        return Err("Session required".to_string());
    };
    let session = Session::open(&session_info.session_id, &session_info.store_id)?;

    // 6. Load user-specific data from session
    let last_query = match session.get("last_query")? {
        Some(TypedValue::AsString(s)) => Some(s),
        _ => None,
    };

    // 7. Send progress notification
    if let Some(stream) = &ctx.client_stream {
        send_log(stream, &ctx.frame,
            format!("Processing request for user {}", user_id));
    }

    // 8. Check HTTP context for tenant
    let tenant_id = if let Some(http_ctx) = &ctx.http_context {
        extract_tenant_header(&http_ctx.headers)
    } else {
        None
    };

    // 9. Perform operation with context
    let result = perform_operation(&request, user_id, tenant_id, last_query)?;

    // 10. Update session
    session.set("last_query", &TypedValue::AsString(request.name.clone()))?;

    Ok(result)
}
```

## Common Patterns

### Pattern: Require Authentication

```rust
fn require_auth(ctx: &MessageContext) -> Result<&Identity, String> {
    ctx.identity.as_ref().ok_or("Authentication required".to_string())
}

fn call_tool(ctx: MessageContext, request: CallToolRequest)
    -> Result<CallToolResult, String>
{
    let identity = require_auth(&ctx)?;
    // ... use identity
}
```

### Pattern: Optional Session

```rust
fn with_optional_session<F, R>(ctx: &MessageContext, f: F) -> Result<R, String>
where
    F: FnOnce(Option<Session>) -> Result<R, String>
{
    let session = ctx.session.as_ref().and_then(|info| {
        Session::open(&info.session_id, &info.store_id).ok()
    });
    f(session)
}
```

### Pattern: Send Notifications Safely

```rust
fn notify(ctx: &MessageContext, level: LogLevel, message: String) {
    if let Some(stream) = &ctx.client_stream {
        let notification = ServerNotification::Log(LoggingMessageNotification {
            data: message,
            level,
            logger: Some("my-tool".to_string()),
        });
        let msg = ServerMessage::Notification(notification);
        let _ = server_io::send_message(stream, msg, &ctx.frame);
    }
}
```

### Pattern: Extract HTTP Header

```rust
fn get_header(ctx: &MessageContext, name: &str) -> Option<String> {
    ctx.http_context.as_ref()?.headers.iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .and_then(|(_, v)| String::from_utf8(v.clone()).ok())
}
```

## Testing

**Mock MessageContext in tests:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn mock_context() -> MessageContext {
        MessageContext {
            client_stream: None,
            protocol_version: "2025-06-18".to_string(),
            session: None,
            identity: None,
            frame: MessageFrame {
                prefix: vec![],
                suffix: vec![],
            },
            http_context: None,
        }
    }

    #[test]
    fn test_tool_without_auth() {
        let ctx = mock_context();
        let request = CallToolRequest {
            name: "test".to_string(),
            arguments: "{}".to_string(),
        };

        let result = call_tool(ctx, request);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Authentication required");
    }
}
```

## Related Documentation

- **[sessions.md](./sessions.md)** - Using session field
- **[authentication-and-authorization.md](./authentication-and-authorization.md)** - Using identity field
- **[typed-value.md](./typed-value.md)** - Session storage types
- **[server.wit](../spec/2025-06-18/wit/server.wit)** - WIT type definitions

## WIT Interface

Full interface: [spec/2025-06-18/wit/server.wit](../spec/2025-06-18/wit/server.wit:27-41)

```wit
interface server-handler {
    record message-context {
        client-stream: option<borrow<output-stream>>,
        protocol-version: string,
        session: option<session>,
        identity: option<identity>,
        frame: message-frame,
        http-context: option<http-context>,
    }

    handle: func(
        ctx: message-context,
        message: client-message,
    ) -> option<result<server-result, error-code>>;
}
```
