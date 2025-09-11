use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Protocol version enum matching MCP specification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProtocolVersion {
    #[serde(rename = "2025-03-26")]
    V20250326,
    #[serde(rename = "2025-06-18")]
    V20250618,
}

impl ProtocolVersion {
    /// Convert to rmcp ProtocolVersion
    pub fn to_rmcp(&self) -> rmcp::model::ProtocolVersion {
        match self {
            ProtocolVersion::V20250326 => rmcp::model::ProtocolVersion::V_2025_03_26,
            ProtocolVersion::V20250618 => rmcp::model::ProtocolVersion::V_2025_06_18,
        }
    }
}

/// Server capabilities matching MCP specification
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerCapabilities {
    #[cfg(feature = "tools")]
    pub tools: Option<ToolsCapability>,
    #[cfg(feature = "resources")]
    pub resources: Option<ResourcesCapability>,
    #[cfg(feature = "prompts")]
    pub prompts: Option<PromptsCapability>,
}

#[cfg(feature = "tools")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    pub list_changed: Option<bool>,
}

#[cfg(feature = "resources")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapability {
    pub subscribe: Option<bool>,
    pub list_changed: Option<bool>,
}

#[cfg(feature = "prompts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapability {
    pub list_changed: Option<bool>,
}

/// Server implementation info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    pub name: String,
    pub version: String,
}

/// Initialize response matching MCP specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    pub protocol_version: ProtocolVersion,
    pub capabilities: ServerCapabilities,
    pub server_info: Implementation,
    pub instructions: Option<String>,
}

/// Content block types for tool responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: Vec<u8>, mime_type: String },
    #[serde(rename = "audio")]
    Audio { data: Vec<u8>, mime_type: String },
    #[cfg(feature = "resources")]
    #[serde(rename = "resource_link")]
    ResourceLink {
        uri: String,
        name: String,
        description: Option<String>,
        mime_type: Option<String>,
        size: Option<u64>,
    },
    #[cfg(feature = "resources")]
    #[serde(rename = "embedded_resource")]
    EmbeddedResource { contents: ResourceContents },
}

#[cfg(feature = "resources")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResourceContents {
    #[serde(rename = "text")]
    Text {
        uri: String,
        mime_type: Option<String>,
        text: String,
    },
    #[serde(rename = "blob")]
    Blob {
        uri: String,
        mime_type: Option<String>,
        blob: Vec<u8>,
    },
}

/// Tool-related types
#[cfg(feature = "tools")]
pub mod tools {
    use super::*;
    use serde_json::Value;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CallToolRequest {
        pub name: String,
        pub arguments: Option<HashMap<String, Value>>,
        pub progress_token: Option<String>,
        pub meta: Option<HashMap<String, String>>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CallToolResponse {
        pub content: Vec<ContentBlock>,
        pub is_error: bool,
        pub meta: Option<HashMap<String, String>>,
        pub structured_content: Option<Value>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Tool {
        pub name: String,
        pub description: Option<String>,
        pub input_schema: Value,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ListToolsResponse {
        pub tools: Vec<Tool>,
        pub next_cursor: Option<String>,
    }
}

/// Resource-related types
#[cfg(feature = "resources")]
pub mod resources {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Resource {
        pub uri: String,
        pub name: String,
        pub description: Option<String>,
        pub mime_type: Option<String>,
        pub size: Option<u64>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ListResourcesResponse {
        pub resources: Vec<Resource>,
        pub next_cursor: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ReadResourceRequest {
        pub uri: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ReadResourceResponse {
        pub contents: ResourceContents,
    }
}

/// Prompt-related types
#[cfg(feature = "prompts")]
pub mod prompts {
    use super::*;
    use serde_json::Value;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Prompt {
        pub name: String,
        pub description: Option<String>,
        pub arguments: Vec<PromptArgument>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PromptArgument {
        pub name: String,
        pub description: Option<String>,
        pub required: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ListPromptsResponse {
        pub prompts: Vec<Prompt>,
        pub next_cursor: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GetPromptRequest {
        pub name: String,
        pub arguments: Option<HashMap<String, Value>>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GetPromptResponse {
        pub messages: Vec<PromptMessage>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PromptMessage {
        pub role: PromptMessageRole,
        pub content: Vec<ContentBlock>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum PromptMessageRole {
        User,
        Assistant,
        System,
    }
}