//! Component introspection and metadata
//!
//! This module handles:
//! - Reading component exports and imports via WIT introspection
//! - Interface naming conventions and constants
//! - Import validation and tracking
//! - Runtime detection from component dependencies

pub mod import_validation;
pub mod interfaces;
pub mod introspection;
pub mod runtime;

pub use import_validation::UnsatisfiedImports;
pub use interfaces::{
    CAPABILITY_INTERFACES, ComponentType, DEFAULT_SPEC_VERSION, InterfaceType, WASMCP_NAMESPACE,
    wasi_cli_run, wasi_http_handler,
};
pub use introspection::{
    check_component_exports, check_component_imports, component_imports_interface,
    find_component_export, get_interface_details,
};
pub use runtime::{RuntimeInfo, RuntimeType, detect_runtime, detect_runtime_from_file};
