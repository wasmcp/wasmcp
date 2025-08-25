//! Minimal WASI SDK for MCP handlers
//! 
//! This provides just the essentials MCP handlers need:
//! - Async HTTP client for outbound requests
//! - Key-value storage
//! - Configuration access

pub mod http;
pub mod keyvalue;
pub mod config;

// Re-export spin_executor for blocking on async operations
pub use spin_executor;

// Generate WASI bindings
#[doc(hidden)]
pub mod wit {
    #![allow(missing_docs)]
    #![allow(warnings)]
    
    wit_bindgen::generate!({
        world: "mcp-wasi",
        path: "./wit",
        with: {
            "wasi:io/error@0.2.0": ::wasi::io::error,
            "wasi:io/streams@0.2.0": ::wasi::io::streams,
            "wasi:io/poll@0.2.0": ::wasi::io::poll,
        },
        generate_all,
    });
}
