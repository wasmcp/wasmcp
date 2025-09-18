// The Guest trait is cargo-component's pattern for implementing WIT interfaces.
// Each exported interface generates a Guest trait that we implement for our Component.
use crate::bindings::exports::wasmcp::mcp::lifecycle::Guest as LifecycleGuest;
use crate::bindings::wasmcp::mcp::{
    types::{Context, McpError, InitializeResult, ServerCapabilities},
};
use crate::Component;

impl LifecycleGuest for Component {
    /// Initialize the MCP server.
    fn initialize(_ctx: &Context) -> Result<InitializeResult, McpError> {
        Ok(InitializeResult::new("weather-rs", "0.1.0", ServerCapabilities::TOOLS | ServerCapabilities::RESOURCES))
    }
}