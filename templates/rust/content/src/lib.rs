//! Transparent MCP provider implementation for {{project-name | kebab_case}}.
//!
//! This implementation uses cargo-component to generate Rust bindings from WIT.
//! Rust's type system maps naturally to the Component Model:
//! - Result<T, E> maps directly to WIT's result<T, E>
//! - Option<T> maps to WIT's option<T>
//! - Guest traits provide the implementation interface

// Generated bindings from cargo-component.
// This module contains all the WIT-generated types and traits.
// The Guest traits define the interfaces we must implement.
#[allow(warnings)]
mod bindings;

mod authorization;
mod lifecycle;
mod tools;

/// The main component struct that implements all Guest traits.
/// This is a zero-sized type - all functionality is in trait implementations.
/// Unlike Go or Python, Rust doesn't need a runtime or event loop;
/// the Component Model runtime directly calls our trait methods.
pub struct Component;

// The export! macro wires up our Guest trait implementations to component exports.
// This generates the actual WebAssembly exports that the runtime will call.
// The 'with_types_in bindings' ensures type definitions come from the bindings module.
bindings::export!(Component with_types_in bindings);