//! MCP Core Library
//! 
//! Shared MCP protocol and authentication implementation for both
//! WebAssembly and native Rust contexts.
//! 
//! This library provides:
//! - Protocol handling and type conversions
//! - JWT validation and JWKS discovery
//! - OAuth 2.0 discovery endpoints
//! - Rego policy evaluation
//! - Runtime abstractions for platform differences

// Note: This library requires std for WebAssembly and native targets
// Both wasm32-wasi and wasm32-wasip1 have std support

pub mod error;
pub mod runtime;
pub mod protocol;
pub mod auth;

// Re-export commonly used types
pub use error::{McpError, AuthError};
