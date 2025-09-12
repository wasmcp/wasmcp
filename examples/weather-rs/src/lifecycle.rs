use crate::bindings::exports::wasmcp::mcp::lifecycle::Guest as LifecycleGuest;
use crate::bindings::wasmcp::mcp::{
    lifecycle_types::{
        Implementation, InitializeRequest, InitializeResult, ServerCapabilities, ToolsCapability,
    },
    mcp_types::McpError,
};
use crate::Component;

impl LifecycleGuest for Component {
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
                name: "weather-rs".to_string(),
                version: "0.1.0".to_string(),
                title: Some("Weather RS Provider".to_string()),
                icons: None,
                website_url: None,
            },
            instructions: Some("A Rust MCP server providing weather tools".to_string()),
        })
    }

    fn client_initialized() -> Result<(), McpError> {
        Ok(())
    }

    fn shutdown() -> Result<(), McpError> {
        Ok(())
    }
}