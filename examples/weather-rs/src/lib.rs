//! MCP provider implementation for weather-rs.

// Generated bindings from cargo-component.
#[allow(warnings)]
mod bindings;

mod authorization;
mod lifecycle;
mod tools;

/// The main component struct that implements all Guest traits.
pub struct Component;

// Wire up Guest trait implementations to component exports.
bindings::export!(Component with_types_in bindings);