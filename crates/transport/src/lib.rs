//! Transport component for the Model Context Protocol (MCP)
//!
//! This transport component provides orchestration for MCP servers:
//! - Delegates I/O parsing/serialization to server-io component
//! - Manages session lifecycle via session-manager (HTTP only)
//! - Coordinates with middleware via server-handler
//! - Handles transport-level MCP methods (initialize, ping, logging/setLevel)
//!
//! Exports both HTTP and CLI interfaces - runtime imports what it needs
//!
//! # Environment Variables
//!
//! ## HTTP Transport Mode
//!
//! - **`WASMCP_DISABLE_SSE`** - Disable Server-Sent Events for HTTP transport
//!   - Default: `false` (SSE enabled - supports streaming and notifications)
//!   - Set to `true` to use plain JSON mode (single response, notifications suppressed)
//!   - Only affects HTTP transport; stdio transport is unaffected
//!
//! ## Session Management
//!
//! - **`WASMCP_SESSION_ENABLED`** - Enable HTTP session support
//!   - Default: `false` (stateless HTTP)
//!   - Set to `true` to enable session tracking via Mcp-Session-Id header
//!
//! - **`WASMCP_SESSION_BUCKET`** - Key-value bucket name for session storage
//!   - Default: `""` (default bucket)
//!   - Used when `WASMCP_SESSION_ENABLED=true`
//!
//! ## Authentication & Authorization
//!
//! - **`WASMCP_AUTH_MODE`** - Authentication mode: `public` (default) or `oauth`
//!   - `public`: No authentication required
//!   - `oauth`: Requires JWT bearer tokens, validates via JWT_* configuration
//!
//! - **`JWT_ISSUER`** - Expected JWT issuer (e.g., `https://auth.example.com`)
//!   - Required when `WASMCP_AUTH_MODE=oauth`
//!
//! - **`JWT_JWKS_URI`** - JWKS endpoint URL for JWT public key retrieval
//!   - Required when `WASMCP_AUTH_MODE=oauth`
//!
//! - **`JWT_AUDIENCE`** - Expected JWT audience claim (server URI)
//!   - Optional: Only required for traditional OAuth pattern
//!   - Do NOT set for dynamic registration flows (e.g., WorkOS) where audience is per-user client ID
//!
//! - **`JWT_PUBLIC_KEY`** - Direct PEM-encoded public key (alternative to JWKS)
//!   - Optional: Use instead of JWT_JWKS_URI for static key validation
//!
//! - **`JWT_REQUIRED_SCOPES`** - Comma-separated list of required OAuth scopes
//!   - Optional: Currently not enforced in dynamic registration flows
//!   - Used in discovery endpoint metadata (/.well-known/oauth-protected-resource)
//!
//! ## Security & CORS
//!
//! - **`WASMCP_ALLOWED_ORIGINS`** - Comma-separated list of allowed Origin header values
//!   - Default: localhost-only (127.0.0.1, ::1)
//!   - Supports `*` wildcard to allow all origins
//!   - Example: `https://app.example.com,https://admin.example.com`
//!   - Prevents DNS rebinding attacks when Origin header is present
//!
//! - **`WASMCP_REQUIRE_ORIGIN`** - Require Origin header on all requests
//!   - Default: `false` (Origin header optional but validated if present)
//!   - Set to `true` to reject requests without Origin header
//!   - NOTE: Most MCP clients (desktop apps) don't send Origin headers
//!   - Only enable if all your clients are browser-based
//!
//! ## Discovery & Metadata
//!
//! - **`WASMCP_SERVER_URI`** - Server's canonical URI (resource identifier)
//!   - Optional: Falls back to Host header if not set
//!   - Used in OAuth discovery metadata and WWW-Authenticate headers
//!   - Example: `https://mcp.example.com`
//!
//! - **`WASMCP_AUTH_SERVER_URL`** - Authorization server URL
//!   - Optional: Falls back to JWT_ISSUER if not set
//!   - Used in OAuth protected resource metadata (RFC 9728)
//!
//! - **`WASMCP_DISCOVERY_CACHE_TTL`** - Cache TTL for discovery endpoints in seconds
//!   - Default: `3600` (1 hour)
//!   - Controls Cache-Control headers on /.well-known/* endpoints

mod bindings {
    wit_bindgen::generate!({
        world: "transport",
        generate_all,
    });
}

mod common;
mod config;
mod error;
mod http;
mod session_keys;
mod stdio;

bindings::export!(Component with_types_in bindings);

struct Component;

// Export HTTP incoming-handler interface
impl bindings::exports::wasi::http::incoming_handler::Guest for Component {
    fn handle(
        request: bindings::wasi::http::types::IncomingRequest,
        response_out: bindings::wasi::http::types::ResponseOutparam,
    ) {
        http::HttpTransportGuest::handle(request, response_out)
    }
}

// Export CLI run interface
impl bindings::exports::wasi::cli::run::Guest for Component {
    fn run() -> Result<(), ()> {
        stdio::StdioTransportGuest::run()
    }
}
