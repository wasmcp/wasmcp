pub mod commands;
pub mod config;
pub mod logging;
pub mod mcp;
pub mod state;
pub mod types;
pub mod versioning;

pub use types::{Language, TemplateType, Transport};

/// Default wasmcp version for framework components
pub const DEFAULT_WASMCP_VERSION: &str = env!("CARGO_PKG_VERSION");
