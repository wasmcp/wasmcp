//! Message serialization module
//!
//! Handles all MCP message serialization to JSON-RPC format.
//! Organized into submodules by message type.

pub mod notifications;
pub mod requests;
pub mod server_messages;

// Re-export main entry point
pub use server_messages::serialize_server_message;
