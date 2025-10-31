//! JSON-RPC and SSE serialization for MCP protocol
//!
//! This module handles conversion from WIT types to JSON-RPC 2.0 format
//! and SSE event formatting.

mod content;
mod responses;
mod types;

// Re-export the main public API functions
pub use responses::{format_sse_event, serialize_jsonrpc_response};
