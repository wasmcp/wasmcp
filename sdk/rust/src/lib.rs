//! Rust SDK for building MCP (Model Context Protocol) WebAssembly components
//!
//! This crate provides an ergonomic, attribute-based API for implementing MCP servers in Rust.
//!
//! # Example
//!
//! ```rust
//! use wasmcp::prelude::*;
//!
//! #[mcp::main]
//! struct MyServer;
//!
//! #[mcp::tool]
//! /// Add two numbers together
//! fn add(a: i32, b: i32) -> Result<i32> {
//!     Ok(a + b)
//! }
//!
//! #[mcp::resource("file://{path}")]
//! /// Read a file from the filesystem  
//! fn read_file(path: String) -> Result<String> {
//!     std::fs::read_to_string(path)
//!         .map_err(|e| e.to_string())
//! }
//! ```

#![warn(missing_docs)]
#![allow(clippy::module_name_repetitions)]

// WIT bindings generated directly
#[doc(hidden)]
pub mod bindings {
    #![allow(warnings)]
    wit_bindgen::generate!({
        world: "rust-handler",
        path: "./wit",
        generate_all,
        additional_derives: [
            serde::Serialize,
            serde::Deserialize,
            Clone,
        ],
    });
}

// Re-export proc macros under mcp namespace
pub mod mcp {
    pub use wasmcp_macros::{main, tool, resource, prompt};
}

// Prelude for convenience imports
pub mod prelude {
    pub use crate::mcp;
    pub use crate::{Result, Error, Prompt};
    pub use serde::{Serialize, Deserialize};
    pub use serde_json::{json, Value};
}

// Re-export commonly used types from bindings
pub use bindings::exports::mcp::protocol::handler::Guest;
pub use bindings::mcp::protocol::session::*;
pub use bindings::mcp::protocol::tools::*;
pub use bindings::mcp::protocol::resources::*;
pub use bindings::mcp::protocol::prompts::*;
pub use bindings::mcp::protocol::types::*;

/// Result type for MCP operations
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Error type for MCP operations
#[derive(Debug)]
pub struct Error {
    message: String,
}

impl Error {
    /// Create a new error with a message
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// A prompt that can be returned by prompt functions
pub struct Prompt {
    messages: Vec<PromptMessage>,
}

impl Prompt {
    /// Create a new prompt with the given messages
    pub fn new(messages: Vec<PromptMessage>) -> Self {
        Self { messages }
    }
    
    /// Convert the prompt into messages for the handler
    pub fn into_messages(self) -> Vec<PromptMessage> {
        self.messages
    }
}

/// Macro for building prompts
#[macro_export]
macro_rules! prompt {
    (system: $system:expr, user: $user:expr $(, $($rest:tt)*)?) => {
        $crate::Prompt::new(vec![
            $crate::PromptMessage {
                role: $crate::Role::User,
                content: vec![$crate::ContentBlock::Text($crate::TextContent {
                    text: format!($system),
                    annotations: None,
                    meta: None,
                })],
                meta: None,
            },
            $crate::PromptMessage {
                role: $crate::Role::User,
                content: vec![$crate::ContentBlock::Text($crate::TextContent {
                    text: format!($user $(, $($rest)*)?),
                    annotations: None,
                    meta: None,
                })],
                meta: None,
            },
        ])
    };
    (user: $user:expr $(, $($rest:tt)*)?) => {
        $crate::Prompt::new(vec![
            $crate::PromptMessage {
                role: $crate::Role::User,
                content: vec![$crate::ContentBlock::Text($crate::TextContent {
                    text: format!($user $(, $($rest)*)?),
                    annotations: None,
                    meta: None,
                })],
                meta: None,
            },
        ])
    };
}

/// Runtime support module (used by proc macros)
#[doc(hidden)]
pub mod runtime {
    use super::*;
    use std::sync::Arc;
    
    /// Registration data for a tool
    pub struct ToolRegistration {
        pub name: String,
        pub description: String,
        pub schema: String,
        pub handler: Box<dyn Fn(serde_json::Value) -> std::result::Result<serde_json::Value, Box<dyn std::error::Error>> + Send + Sync>,
    }
    
    /// Registration data for a resource
    pub struct ResourceRegistration {
        pub name: String,
        pub uri_pattern: String,
        pub description: String,
        pub mime_type: Option<String>,
        pub handler: Box<dyn Fn(String) -> std::result::Result<String, Box<dyn std::error::Error>> + Send + Sync>,
    }
    
    /// Registration data for a prompt
    pub struct PromptRegistration {
        pub name: String,
        pub description: String,
        pub handler: Box<dyn Fn(serde_json::Value) -> std::result::Result<Vec<PromptMessage>, Box<dyn std::error::Error>> + Send + Sync>,
    }
    
    /// Check if a URI matches a pattern
    pub fn uri_matches(pattern: &str, uri: &str) -> bool {
        // Simple implementation - in production this would handle {param} extraction
        if !pattern.contains('{') {
            return pattern == uri;
        }
        
        // Very basic pattern matching
        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let uri_parts: Vec<&str> = uri.split('/').collect();
        
        if pattern_parts.len() != uri_parts.len() {
            return false;
        }
        
        for (p, u) in pattern_parts.iter().zip(uri_parts.iter()) {
            if p.starts_with('{') && p.ends_with('}') {
                // This is a parameter, accept any value
                continue;
            }
            if p != u {
                return false;
            }
        }
        
        true
    }
}