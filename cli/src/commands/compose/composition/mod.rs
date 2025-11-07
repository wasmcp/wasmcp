//! Core composition logic
//!
//! This module handles the actual WebAssembly component composition:
//! - Graph building using wac-graph
//! - Component wiring (middleware chains, transport connections)
//! - Package loading and registration
//! - Capability wrapping (auto-detecting and wrapping tools/resources/prompts)

pub mod graph;
pub mod packaging;
pub mod wiring;
pub mod wrapping;

pub use graph::{build_composition, build_handler_composition};
pub use packaging::{CompositionPackages, load_and_register_components, load_package};
pub use wiring::{build_middleware_chain, wire_transport};
pub use wrapping::wrap_capabilities;
