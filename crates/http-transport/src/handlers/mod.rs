//! HTTP request handlers for MCP transport
//!
//! Organized into:
//! - `http` - POST, GET, DELETE request handlers
//! - `jsonrpc` - JSON-RPC request/notification/response handlers
//! - `initialize` - Initialize request handling (separate due to complexity)
//! - `transport` - Transport-level methods (ping, setLevel)

pub mod http;
pub mod initialize;
pub mod jsonrpc;
pub mod transport;

// Re-export main entry points
pub use http::{handle_delete, handle_get, handle_post};
pub use jsonrpc::{handle_json_rpc_notification, handle_json_rpc_request, handle_json_rpc_response};
pub use transport::{handle_ping_request, handle_set_level_request};
