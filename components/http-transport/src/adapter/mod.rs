// Core adapter for converting between WIT and RMCP types
use anyhow::Result;

/// Adapter that handles conversions between WIT bindings and RMCP types
pub struct WitMcpAdapter;

impl WitMcpAdapter {
    pub fn new() -> Self {
        Self
    }
    
    /// Convert WIT InitializeResponse to rmcp ServerInfo
    /// The transport determines protocol version and capabilities based on compile-time features
    pub fn convert_initialize_to_rmcp(
        &self,
        response: crate::bindings::wasmcp::mcp::lifecycle_types::InitializeResult,
    ) -> Result<rmcp::model::ServerInfo> {
        use rmcp::model::{Implementation, ServerCapabilities, ServerInfo, ProtocolVersion};

        // Transport determines the protocol version (fixed at compile time)
        let protocol_version = ProtocolVersion::V_2025_06_18;

        // Transport determines capabilities based on compile-time features
        let capabilities = ServerCapabilities {
            #[cfg(feature = "tools")]
            tools: Some(rmcp::model::ToolsCapability {
                list_changed: Some(false),
            }),
            #[cfg(not(feature = "tools"))]
            tools: None,

            #[cfg(feature = "resources")]
            resources: Some(rmcp::model::ResourcesCapability {
                subscribe: None,
                list_changed: Some(false),
            }),
            #[cfg(not(feature = "resources"))]
            resources: None,

            #[cfg(feature = "prompts")]
            prompts: Some(rmcp::model::PromptsCapability {
                list_changed: Some(false),
            }),
            #[cfg(not(feature = "prompts"))]
            prompts: None,

            #[cfg(feature = "completion")]
            completions: Some(serde_json::Map::new()),
            #[cfg(not(feature = "completion"))]
            completions: None,

            ..Default::default()
        };

        Ok(ServerInfo {
            protocol_version,
            capabilities,
            server_info: Implementation {
                name: response.server_info.name,
                version: response.server_info.version,
            },
            instructions: response.instructions,
        })
    }
}

// Capability-specific conversions
pub mod lifecycle;

#[cfg(feature = "tools")]
pub mod tools;

#[cfg(feature = "resources")]
pub mod resources;

#[cfg(feature = "prompts")]
pub mod prompts;