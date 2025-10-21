//! Configuration management for wasmcp
//!
//! This module provides a centralized, extensible configuration system for wasmcp.
//! All config operations should go through this module to ensure validation and consistency.
//!
//! ## Architecture
//!
//! - `schema` - Configuration data structures
//! - `io` - Reading, writing, and updating config files
//! - `paths` - Directory path management
//!
//! ## Usage
//!
//! ```rust
//! use wasmcp::config;
//!
//! # fn example() -> anyhow::Result<()> {
//! // Load config (returns default if file doesn't exist)
//! let config = config::io::load_config()?;
//!
//! // Register a component alias
//! config::io::register_component("calc", "wasmcp:calculator@0.1.0")?;
//!
//! // Get paths
//! let deps_dir = config::paths::get_deps_dir()?;
//! # Ok(())
//! # }
//! ```

pub mod io;
pub mod paths;
pub mod schema;
pub mod utils;

// Re-export commonly used items
pub use io::{
    create_profile, delete_profile, load_config, register_component, unregister_component,
};
pub use paths::{
    ensure_dirs, get_cache_dir, get_composed_dir, get_config_path, get_deps_dir, get_wasmcp_dir,
};
pub use schema::{Profile, WasmcpConfig};
