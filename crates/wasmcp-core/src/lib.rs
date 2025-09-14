pub mod traits;
pub mod handlers;

// Generate types only, skip import glue code but keep export types
wit_bindgen::generate!({
    world: "types",
    path: "wit",
    additional_derives: [serde::Serialize, serde::Deserialize, Clone],
    generate_unused_types: true,
});

// Types are now generated directly at the root level by wit_bindgen
// ErrorCode and McpError are available automatically

pub use wasmcp::mcp::mcp_types::{ErrorCode, McpError};

pub use traits::McpLifecycleHandler;
pub use handlers::lifecycle;
