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

#[cfg(feature = "draft2")]
mod bindings {
    wit_bindgen::generate!({
        path: "wit-draft2",
        world: "sessions-draft2",
        generate_all,
    });
}

#[cfg(not(feature = "draft2"))]
mod bindings {
    wit_bindgen::generate!({
        world: "sessions",
        generate_all,
    });
}

mod session;

use bindings::exports::wasmcp::mcp_v20250618::session_manager::Guest as SessionManagerGuest;
use bindings::exports::wasmcp::mcp_v20250618::sessions::Guest as SessionsGuest;

struct Component;

// Export user-facing sessions interface
impl SessionsGuest for Component {
    type Session = session::SessionImpl;
    type FutureElicitResult = session::FutureElicitResultImpl;
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
}

bindings::export!(Component with_types_in bindings);
