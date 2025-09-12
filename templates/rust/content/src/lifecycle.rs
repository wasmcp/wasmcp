// The Guest trait is cargo-component's pattern for implementing WIT interfaces.
// Each exported interface generates a Guest trait that we implement for our Component.
use crate::bindings::exports::wasmcp::mcp::lifecycle::Guest as LifecycleGuest;
use crate::bindings::wasmcp::mcp::{
    lifecycle_types::{
        Implementation, InitializeRequest, InitializeResult, ServerCapabilities, ToolsCapability,
    },
    mcp_types::McpError,
};
use crate::Component;

impl LifecycleGuest for Component {
    /// Initialize the MCP server.
    ///
    /// Rust's Result<T, E> maps directly to WIT's result<T, E>.
    /// Unlike Go which needs special Result wrapper types, or Python which
    /// hides the Result handling, Rust's native Result is a perfect match
    /// for the Component Model's error handling.
    fn initialize(_request: InitializeRequest) -> Result<InitializeResult, McpError> {
        Ok(InitializeResult {
            protocol_version: "0.1.0".to_string(),
            capabilities: ServerCapabilities {
                experimental: None,
                logging: None,
                completions: None,
                prompts: None,
                resources: None,
                tools: Some(ToolsCapability {
                    list_changed: None,
                }),
            },
            server_info: Implementation {
                name: "{{project-name | kebab_case}}".to_string(),
                version: "0.1.0".to_string(),
                title: Some("{{project-name | kebab_case}} Provider".to_string()),
                icons: None,
                website_url: None,
            },
            instructions: Some("{{project-description}}".to_string()),
        })
    }

    /// Called when the client has initialized.
    ///
    /// Note: These are associated functions (no self parameter) because
    /// the Component Model is stateless - each call is independent.
    fn client_initialized() -> Result<(), McpError> {
        Ok(())
    }

    /// Shutdown the server.
    ///
    /// The Component Model manages the component lifecycle. This method
    /// is called by the runtime, not by dropping the Component struct.
    fn shutdown() -> Result<(), McpError> {
        Ok(())
    }
}