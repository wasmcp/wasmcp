//! MCP server implementation for wasmcp
//!
//! This module provides the Model Context Protocol (MCP) server functionality
//! for wasmcp, including resource management, tool execution, and protocol handling.

pub mod resources;
pub mod server;
pub mod tools;

// Re-export the main server struct for convenience
pub use server::WasmcpServer;
