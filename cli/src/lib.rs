pub mod commands;
pub mod config;
pub mod logging;
pub mod types;

pub use types::{Language, TemplateType, Transport};

/// Default version for wasmcp WIT dependencies
/// This should match the wasmcp package version in most cases
pub const DEFAULT_WASMCP_VERSION: &str = "0.1.0-beta.2";
