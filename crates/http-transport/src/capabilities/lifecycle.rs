use crate::bindings;
use crate::error::{ErrorCode, McpError};
use rmcp::model::{InitializeRequestParam, InitializeResult};
use serde_json::Value;

pub fn initialize(params: Option<Value>) -> Result<Value, McpError> {
    // Convert JSON-RPC params to WIT request
    let wit_request = if let Some(p) = params {
        // Parse the incoming params
        let params: InitializeRequestParam =
            serde_json::from_value(p).map_err(|e| McpError {
                code: ErrorCode::InvalidParams,
                message: format!("Invalid params: {e}"),
                data: None,
            })?;
        
        // Convert rmcp InitializeRequestParam to WIT types
        bindings::wasmcp::mcp::lifecycle_types::InitializeRequest {
            protocol_version: params.protocol_version.to_string(),
            capabilities: bindings::wasmcp::mcp::lifecycle_types::ClientCapabilities {
                experimental: params.capabilities.experimental.map(|exp| {
                    serde_json::to_string(&exp).unwrap_or_else(|_| "{}".to_string())
                }),
                roots: params.capabilities.roots.map(|r| {
                    bindings::wasmcp::mcp::lifecycle_types::RootsCapability {
                        list_changed: r.list_changed,
                    }
                }),
                sampling: params.capabilities.sampling.map(|s| {
                    serde_json::to_string(&s).unwrap_or_else(|_| "{}".to_string())
                }),
                elicitation: params.capabilities.elicitation.map(|e| {
                    bindings::wasmcp::mcp::lifecycle_types::ElicitationCapability {
                        schema_validation: e.schema_validation,
                    }
                }),
            },
            client_info: bindings::wasmcp::mcp::lifecycle_types::Implementation {
                name: params.client_info.name,
                title: params.client_info.title,
                version: params.client_info.version,
                website_url: params.client_info.website_url,
                icons: params.client_info.icons.map(|icons| {
                    icons.into_iter().map(|i| bindings::wasmcp::mcp::mcp_types::Icon {
                        src: i.src,
                        mime_type: i.mime_type,
                        sizes: i.sizes,
                    }).collect()
                }),
            },
        }
    } else {
        // Default request when no params provided
        bindings::wasmcp::mcp::lifecycle_types::InitializeRequest {
            protocol_version: "2025-06-18".to_string(),
            capabilities: bindings::wasmcp::mcp::lifecycle_types::ClientCapabilities {
                experimental: None,
                roots: None,
                sampling: None,
                elicitation: None,
            },
            client_info: bindings::wasmcp::mcp::lifecycle_types::Implementation {
                name: "unknown".to_string(),
                version: "0.0.0".to_string(),
                title: None,
                website_url: None,
                icons: None,
            },
        }
    };

    // Call the provider's initialize handler
    let response = bindings::wasmcp::mcp::lifecycle::initialize(&wit_request)?;
    
    // Convert WIT InitializeResult to rmcp InitializeResult
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
    
    Ok(serde_json::to_value(result).unwrap())
}

pub fn client_initialized() -> Result<Value, McpError> {
    // Call the provider's client-initialized handler
    bindings::wasmcp::mcp::lifecycle::client_initialized()?;
    Ok(Value::Null)
}

pub fn shutdown() -> Result<Value, McpError> {
    // Call the provider's shutdown handler
    bindings::wasmcp::mcp::lifecycle::shutdown()?;
    Ok(serde_json::json!({}))
}