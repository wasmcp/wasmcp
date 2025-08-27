/// Null resources implementation - provides empty resource capabilities
/// This component satisfies resource-handler requirements with no-op implementations

#[allow(warnings)]
mod bindings;

use bindings::exports::fastertools::mcp::{core, resource_handler};
use bindings::fastertools::mcp::{
    resources::{
        ListResourcesRequest, ListResourcesResponse,
        ListTemplatesRequest, ListTemplatesResponse,
        ReadResourceRequest, ReadResourceResponse,
        SubscribeRequest, UnsubscribeRequest,
    },
    session::{InitializeRequest, InitializeResponse, ServerCapabilities, ImplementationInfo},
    types::{McpError, ErrorCode},
};

pub struct Component;

// Implement core interface
impl core::Guest for Component {
    fn handle_initialize(_request: InitializeRequest) -> Result<InitializeResponse, McpError> {
        Ok(InitializeResponse {
            protocol_version: "2025-06-18".to_string(),
            capabilities: ServerCapabilities {
                // We provide resources capability (even though it's empty)
                resources: Some(bindings::fastertools::mcp::session::ResourcesCapability {
                    subscribe: None,
                    list_changed: None,
                }),
                // No other capabilities from this null component
                tools: None,
                prompts: None,
                logging: None,
                completions: None,
                experimental: None,
            },
            server_info: ImplementationInfo {
                name: "null-resources".to_string(),
                version: "0.1.0".to_string(),
                title: Some("Null Resources Provider".to_string()),
            },
            instructions: Some("This is a null resources provider - no resources available".to_string()),
            meta: None,
        })
    }
    
    fn handle_initialized() -> Result<(), McpError> {
        Ok(())
    }
    
    fn handle_ping() -> Result<(), McpError> {
        Ok(())
    }
    
    fn handle_shutdown() -> Result<(), McpError> {
        Ok(())
    }
}

// Implement resource handler with null behavior
impl resource_handler::Guest for Component {
    fn handle_list_resources(_request: ListResourcesRequest) -> Result<ListResourcesResponse, McpError> {
        // Always return empty list
        Ok(ListResourcesResponse {
            resources: vec![],
            next_cursor: None,
            meta: None,
        })
    }
    
    fn handle_list_resource_templates(_request: ListTemplatesRequest) -> Result<ListTemplatesResponse, McpError> {
        // No templates available
        Ok(ListTemplatesResponse {
            templates: vec![],
            next_cursor: None,
            meta: None,
        })
    }
    
    fn handle_read_resource(_request: ReadResourceRequest) -> Result<ReadResourceResponse, McpError> {
        // Always return "not found" for any resource request
        Err(McpError {
            code: ErrorCode::ResourceNotFound,
            message: "No resources available from null provider".to_string(),
            data: None,
        })
    }
    
    fn handle_subscribe_resource(_request: SubscribeRequest) -> Result<(), McpError> {
        // Accept subscription but it's a no-op
        Ok(())
    }
    
    fn handle_unsubscribe_resource(_request: UnsubscribeRequest) -> Result<(), McpError> {
        // Accept unsubscription but it's a no-op
        Ok(())
    }
}

bindings::export!(Component with_types_in bindings);