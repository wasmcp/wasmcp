use super::WitMcpAdapter;
use anyhow::Result;
use rmcp::model::InitializeRequestParam;

impl WitMcpAdapter {
    /// Convert rmcp InitializeRequestParam to WIT InitializeRequest
    pub fn convert_initialize_request(
        &self,
        params: InitializeRequestParam,
    ) -> Result<crate::bindings::wasmcp::mcp::lifecycle_types::InitializeRequest> {
        use crate::bindings::wasmcp::mcp::lifecycle_types::{
            ClientCapabilities as WitClientCapabilities,
            ImplementationInfo, InitializeRequest, ProtocolVersion, RootsCapability,
        };

        // Convert client capabilities
        let capabilities = WitClientCapabilities {
            // Convert experimental capabilities to meta fields if present
            experimental: params.capabilities.experimental.and_then(|exp| {
                // Convert the experimental map to Vec<(String, String)> meta fields
                let fields: Vec<(String, String)> = exp
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string())))
                    .collect();
                if fields.is_empty() {
                    None
                } else {
                    Some(fields)
                }
            }),
            // Convert roots capability
            roots: params.capabilities.roots.map(|r| RootsCapability {
                list_changed: r.list_changed,
            }),
            // Sampling is present if the capability exists (even if empty)
            sampling: params.capabilities.sampling.map(|_| true),
            // Elicitation is present if the capability exists
            elicitation: params.capabilities.elicitation.map(|_| true),
        };

        // Convert client info (rmcp Implementation doesn't have title field)
        let client_info = ImplementationInfo {
            name: params.client_info.name,
            version: params.client_info.version,
            title: None, // rmcp doesn't have title field
        };

        // Convert protocol version - for now we only support one version
        // In the future we could check params.protocol_version.as_str() to handle multiple versions
        let protocol_version = ProtocolVersion::V20250618;

        Ok(InitializeRequest {
            protocol_version,
            capabilities,
            client_info,
            meta: None, // rmcp doesn't have meta on InitializeRequestParam
        })
    }
}