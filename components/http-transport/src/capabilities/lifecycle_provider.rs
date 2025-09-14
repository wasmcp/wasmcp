use crate::bindings;
use rmcp::model::{InitializeRequestParam, InitializeResult};
use wasmcp_core::{
    McpLifecycleHandler, McpError, InitializeRequest, ClientCapabilities, RootsCapability,
    ElicitationCapability, Implementation, Icon
};

/// A concrete implementation of the lifecycle provider that communicates with the WASM host
/// through the generated WIT bindings.
pub struct LifecycleProvider;

impl McpLifecycleHandler for LifecycleProvider {
    /// Handle the initialize request by converting rmcp types to WIT types,
    /// calling the WIT binding, and converting the response back.
    fn initialize(&self, params: InitializeRequestParam) -> Result<InitializeResult, McpError> {
        // Convert the generic request struct into the wasmcp_core request struct
        let wit_request = InitializeRequest {
            protocol_version: params.protocol_version.to_string(),
            capabilities: ClientCapabilities {
                experimental: params.capabilities.experimental.map(|exp| {
                    serde_json::to_string(&exp).unwrap_or_else(|_| "{}".to_string())
                }),
                roots: params.capabilities.roots.map(|r| {
                    RootsCapability {
                        list_changed: r.list_changed,
                    }
                }),
                sampling: params.capabilities.sampling.map(|s| {
                    serde_json::to_string(&s).unwrap_or_else(|_| "{}".to_string())
                }),
                elicitation: params.capabilities.elicitation.map(|e| {
                    ElicitationCapability {
                        schema_validation: e.schema_validation,
                    }
                }),
            },
            client_info: Implementation {
                name: params.client_info.name,
                title: params.client_info.title,
                version: params.client_info.version,
                website_url: params.client_info.website_url,
                icons: params.client_info.icons.map(|icons| {
                    icons.into_iter().map(|i| Icon {
                        src: i.src,
                        mime_type: i.mime_type,
                        sizes: i.sizes,
                    }).collect()
                }),
            },
        };

        // Call the external function defined by the WIT binding
        let response = bindings::wasmcp::transport::lifecycle::initialize(&wit_request)?;

        // Convert the WIT-generated result struct back into the generic result struct
        let result = InitializeResult {
            protocol_version: match response.protocol_version.as_str() {
                "2025-06-18" => rmcp::model::ProtocolVersion::V_2025_06_18,
                "2025-03-26" => rmcp::model::ProtocolVersion::V_2025_03_26,
                "2024-11-05" => rmcp::model::ProtocolVersion::V_2024_11_05,
                _ => rmcp::model::ProtocolVersion::LATEST,
            },
            capabilities: rmcp::model::ServerCapabilities {
                experimental: response.capabilities.experimental.and_then(|exp| {
                    serde_json::from_str(&exp).ok()
                }),
                logging: response.capabilities.logging.and_then(|log| {
                    serde_json::from_str(&log).ok()
                }),
                completions: response.capabilities.completions.and_then(|comp| {
                    serde_json::from_str(&comp).ok()
                }),
                prompts: response.capabilities.prompts.map(|p| rmcp::model::PromptsCapability {
                    list_changed: p.list_changed,
                }),
                resources: response.capabilities.resources.map(|r| rmcp::model::ResourcesCapability {
                    subscribe: r.subscribe,
                    list_changed: r.list_changed,
                }),
                tools: response.capabilities.tools.map(|t| rmcp::model::ToolsCapability {
                    list_changed: t.list_changed,
                }),
            },
            server_info: rmcp::model::Implementation {
                name: response.server_info.name,
                title: response.server_info.title,
                version: response.server_info.version,
                website_url: response.server_info.website_url,
                icons: response.server_info.icons.map(|icons| {
                    icons.into_iter().map(|i| rmcp::model::Icon {
                        src: i.src,
                        mime_type: i.mime_type,
                        sizes: i.sizes,
                    }).collect()
                }),
            },
            instructions: response.instructions,
        };

        Ok(result)
    }

    /// Handle the client_initialized notification
    fn client_initialized(&self) -> Result<(), McpError> {
        bindings::wasmcp::transport::lifecycle::client_initialized()?;
        Ok(())
    }

    /// Handle the shutdown request
    fn shutdown(&self) -> Result<(), McpError> {
        bindings::wasmcp::transport::lifecycle::shutdown()?;
        Ok(())
    }
}