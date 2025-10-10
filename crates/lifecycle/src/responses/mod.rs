//! Response writer implementations
//!
//! This module contains all MCP response serializers that convert protocol
//! types into JSON-RPC 2.0 formatted messages and write them via output::write-message().

pub mod error;
pub mod lifecycle;
pub mod notifications;

// TODO: Implement full response writers for feature-specific responses
// For now, these are exported but not yet implemented
pub mod completions;
pub mod prompts;
pub mod resources;
pub mod tools;
