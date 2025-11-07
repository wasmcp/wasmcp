//! Build configuration and validation
//!
//! This module handles:
//! - ComposeOptionsBuilder pattern for programmatic API
//! - Input validation (transport types, paths, permissions)
//! - Profile resolution and expansion

pub mod builder;
pub mod profiles;
pub mod validation;

pub use builder::ComposeOptionsBuilder;
pub use profiles::{expand_profile_specs, resolve_profile};
pub use validation::{resolve_output_path, validate_output_file, validate_transport};
