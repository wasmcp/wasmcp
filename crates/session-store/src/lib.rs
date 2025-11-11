//! Sessions component for MCP
//!
//! Exports both the `wasmcp:mcp-v20250618/sessions@0.1.4` interface (user-facing data API)
//! and the `wasmcp:mcp-v20250618/session-manager@0.1.4` interface (transport-facing lifecycle API).
//!
//! This component follows the auto-composition pattern where the CLI detects
//! components that import the sessions or session-manager interfaces and automatically
//! includes the sessions component during composition.
//!
//! ## Architecture
//! - Exports BOTH interfaces for different consumers:
//!   - sessions: User tools import for data access (get/set)
//!   - session-manager: Transports import for lifecycle (initialize, validate, terminate, delete)
//! - Stores session data in WASI KV with session ID as the top-level key
//! - Internal storage format: { "__meta__": {...}, "data": {...} }
//! - Generates UUIDs using wasi:random for session IDs

mod bindings {
    wit_bindgen::generate!({
        world: "sessions",
        generate_all,
    });
}

mod oauth_helpers;
mod session;

use bindings::exports::wasmcp::mcp_v20250618::session_manager::Guest as SessionManagerGuest;
use bindings::exports::wasmcp::mcp_v20250618::sessions::Guest as SessionsGuest;

struct Component;

// Export user-facing sessions interface
impl SessionsGuest for Component {
    type Session = session::SessionImpl;
    type FutureElicitResult = session::FutureElicitResultImpl;

    // OAuth claim helpers (NO parse_claims - claims arrive structured!)
    fn flatten_claims(claims: bindings::wasmcp::oauth::types::JwtClaims) -> Vec<(String, String)> {
        oauth_helpers::flatten_claims(&claims)
    }

    fn has_scope(claims: bindings::wasmcp::oauth::types::JwtClaims, scope: String) -> bool {
        oauth_helpers::has_scope(&claims, &scope)
    }

    fn has_any_scope(
        claims: bindings::wasmcp::oauth::types::JwtClaims,
        scopes: Vec<String>,
    ) -> bool {
        oauth_helpers::has_any_scope(&claims, &scopes)
    }

    fn has_all_scopes(
        claims: bindings::wasmcp::oauth::types::JwtClaims,
        scopes: Vec<String>,
    ) -> bool {
        oauth_helpers::has_all_scopes(&claims, &scopes)
    }

    fn get_claim(claims: bindings::wasmcp::oauth::types::JwtClaims, key: String) -> Option<String> {
        oauth_helpers::get_claim(&claims, &key)
    }

    fn has_audience(claims: bindings::wasmcp::oauth::types::JwtClaims, audience: String) -> bool {
        oauth_helpers::has_audience(&claims, &audience)
    }

    fn is_expired(
        claims: bindings::wasmcp::oauth::types::JwtClaims,
        clock_skew_seconds: Option<u64>,
    ) -> bool {
        oauth_helpers::is_expired(&claims, clock_skew_seconds)
    }

    fn is_valid_time(
        claims: bindings::wasmcp::oauth::types::JwtClaims,
        clock_skew_seconds: Option<u64>,
    ) -> bool {
        oauth_helpers::is_valid_time(&claims, clock_skew_seconds)
    }

    fn get_subject(claims: bindings::wasmcp::oauth::types::JwtClaims) -> String {
        oauth_helpers::get_subject(&claims)
    }

    fn get_issuer(claims: bindings::wasmcp::oauth::types::JwtClaims) -> Option<String> {
        oauth_helpers::get_issuer(&claims)
    }

    fn get_scopes(claims: bindings::wasmcp::oauth::types::JwtClaims) -> Vec<String> {
        oauth_helpers::get_scopes(&claims)
    }

    fn get_audiences(claims: bindings::wasmcp::oauth::types::JwtClaims) -> Vec<String> {
        oauth_helpers::get_audiences(&claims)
    }
}

// Export transport-facing session-manager interface
impl SessionManagerGuest for Component {
    fn initialize(
        store_id: String,
    ) -> Result<String, bindings::exports::wasmcp::mcp_v20250618::session_manager::SessionError>
    {
        session::SessionManager::initialize(store_id)
    }

    fn validate(
        session_id: String,
        store_id: String,
    ) -> Result<bool, bindings::exports::wasmcp::mcp_v20250618::session_manager::SessionError> {
        session::SessionManager::validate(session_id, store_id)
    }

    fn mark_terminated(
        session_id: String,
        store_id: String,
        reason: Option<String>,
    ) -> Result<(), bindings::exports::wasmcp::mcp_v20250618::session_manager::SessionError> {
        session::SessionManager::mark_terminated(session_id, store_id, reason)
    }

    fn delete_session(
        session_id: String,
        store_id: String,
    ) -> Result<(), bindings::exports::wasmcp::mcp_v20250618::session_manager::SessionError> {
        session::SessionManager::delete_session(session_id, store_id)
    }

    fn set_expiration(
        session_id: String,
        store_id: String,
        expires_at: u64,
    ) -> Result<(), bindings::exports::wasmcp::mcp_v20250618::session_manager::SessionError> {
        session::SessionManager::set_expiration(session_id, store_id, expires_at)
    }
}

bindings::export!(Component with_types_in bindings);
