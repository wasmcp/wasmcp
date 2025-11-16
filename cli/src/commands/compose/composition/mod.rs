//! Core composition logic
//!
//! This module handles the actual WebAssembly component composition:
//! - Graph building using wac-graph
//! - Component wiring (middleware chains, transport connections)
//! - Package loading and registration
//! - Capability wrapping (auto-detecting and wrapping tools/resources/prompts)
//! - Service registry for automatic import/export discovery and wiring

pub mod graph;
pub mod packaging;
pub mod service_registry;
pub mod wiring;
pub mod wrapping;

pub use graph::{build_composition, build_handler_composition};
pub use packaging::{CompositionPackages, load_and_register_components, load_package};
pub use service_registry::{ServiceInfo, ServiceRegistry};
pub use wiring::{build_middleware_chain, wire_all_services, wire_if_imports, wire_transport};
pub use wrapping::wrap_capabilities;
