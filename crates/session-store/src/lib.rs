//! Sessions component for MCP
//!
//! Exports the `wasmcp:mcp-v20250618/sessions@0.1.3` interface, enabling tools
//! to import and use session management capabilities.
//!
//! This component follows the auto-composition pattern where the CLI detects
//! components that import the sessions interface and automatically includes
//! the sessions component during composition.
//!
//! ## Architecture
//! - Session IDs come from http-transport via RequestCtx (pre-validated)
//! - Stores session data in WASI KV with session ID as the top-level key
//! - Provides get/set methods for application data storage
//! - Supports session lifecycle (open, delete, terminate, is-terminated)

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

use bindings::exports::wasmcp::mcp_v20250618::sessions::Guest;

struct Sessions;

impl Guest for Sessions {
    type Session = session::SessionImpl;
    type FutureElicitResult = session::FutureElicitResultImpl;
}

bindings::export!(Sessions with_types_in bindings);
