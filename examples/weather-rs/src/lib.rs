//! Transparent MCP provider implementation for weather-rs.
//!
//! This implementation uses WIT bindings directly as the SDK, without
//! abstraction layers.

#[allow(warnings)]
mod bindings;

mod authorization;
mod lifecycle;
mod tools;

/// The main component struct required by the WIT bindings.
pub struct Component;

// Export the WIT bindings
bindings::export!(Component with_types_in bindings);