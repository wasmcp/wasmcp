//! Tool result component for the Model Context Protocol (MCP)
//!
//! This component provides writer implementations for tool-related MCP responses.
//! It supports:
//! - Tool listing with pagination
//! - Tool call results with multiple content types
//! - Structured data responses
//! - Streaming large responses with backpressure
//!
//! The component is organized into modules:
//! - `list_writer`: Writer for tool listings with pagination
//! - `content_writer`: Writer for content blocks (text, images, audio, etc.)
//! - `structured_writer`: Writer for structured data responses
//! - `helpers`: Utility functions for JSON conversion and streaming
//! - `types`: Type definitions for MCP response structures

mod bindings {
    wit_bindgen::generate!({
        world: "tools-writer",
        generate_all,
    });
}

mod content_writer;
mod helpers;
mod list_writer;
mod structured_writer;
mod types;

/// The main component struct for the WASM export.
pub struct Component;

// Export the component
bindings::export!(Component with_types_in bindings);
