//! Resource result component for the Model Context Protocol (MCP)
//!
//! This component provides writer implementations for resource-related MCP responses.
//! It supports:
//! - Resource listing with pagination
//! - Resource template listing
//! - Resource content reading
//! - Streaming large responses with backpressure
//!
//! The component is organized into modules:
//! - `list_writer`: Writer for resource listings with pagination
//! - `templates_list_writer`: Writer for resource template listings
//! - `content_writer`: Writer for resource contents
//! - `helpers`: Utility functions for JSON conversion and streaming

mod bindings {
    wit_bindgen::generate!({
        world: "resources-writer",
        generate_all,
    });
}

mod content_writer;
mod helpers;
mod list_writer;
mod templates_list_writer;

/// The main component struct for the WASM export.
pub struct Component;

// Export the component
bindings::export!(Component with_types_in bindings);
