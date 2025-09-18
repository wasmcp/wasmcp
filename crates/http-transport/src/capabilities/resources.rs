use crate::bindings;
use crate::error::{ErrorCode, McpError};
use rmcp::model::{ListResourcesResult, PaginatedRequestParam, ReadResourceResult};
use serde_json::Value;

pub fn list_resources(params: Option<Value>) -> Result<Value, McpError> {
    let _params: Option<PaginatedRequestParam> = params
        .map(|p| serde_json::from_value(p))
        .transpose()
        .map_err(|e| McpError {
            code: ErrorCode::InvalidParams,
            message: format!("Invalid params: {}", e),
            data: None,
        })?;

    let request = bindings::wasmcp::mcp::resources_types::ListResourcesRequest {
        cursor: None,
    };

    let response = bindings::wasmcp::mcp::resources::list_resources(&request)?;
    
    // Convert WIT ListResourcesResult to rmcp
    let result = ListResourcesResult {
        resources: response.resources.into_iter().map(|r| {
            rmcp::model::Resource::new(rmcp::model::RawResource {
                uri: r.uri,
                name: r.name,
                title: r.title,
                description: r.description,
                mime_type: r.mime_type,
                size: r.size,
                icons: r.icons.map(|icons| {
                    icons.into_iter().map(|i| rmcp::model::Icon {
                        src: i.src,
                        mime_type: i.mime_type,
                        sizes: i.sizes,
                    }).collect()
                }),
            }, r.annotations.map(|a| rmcp::model::Annotations {
                audience: a.audience.map(|roles| {
                    roles.into_iter().map(|role| match role {
                        bindings::wasmcp::mcp::mcp_types::Role::User => rmcp::model::Role::User,
                        bindings::wasmcp::mcp::mcp_types::Role::Assistant => rmcp::model::Role::Assistant,
                    }).collect()
                }),
                priority: a.priority.map(|p| p as f32),
                last_modified: a.last_modified.and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                }),
            }))
        }).collect(),
        next_cursor: response.next_cursor,
    };
    
    Ok(serde_json::to_value(result).unwrap())
}

pub fn read_resource(params: Option<Value>) -> Result<Value, McpError> {
    let params = params.ok_or_else(|| McpError {
        code: ErrorCode::InvalidParams,
        message: "Missing params".to_string(),
        data: None,
    })?;

    let uri = params["uri"].as_str().ok_or_else(|| McpError {
        code: ErrorCode::InvalidParams,
        message: "Missing uri parameter".to_string(),
        data: None,
    })?;

    let request = bindings::wasmcp::mcp::resources_types::ReadResourceRequest {
        uri: uri.to_string(),
    };

    let response = bindings::wasmcp::mcp::resources::read_resource(&request)?;
    
    // Convert WIT ReadResourceResult to rmcp
    let result = ReadResourceResult {
        contents: response.contents.into_iter().map(|c| {
            use bindings::wasmcp::mcp::mcp_types::ResourceContents;
            match c {
                ResourceContents::Text(t) => rmcp::model::ResourceContents::TextResourceContents {
                    uri: t.uri,
                    mime_type: t.mime_type,
                    text: t.text,
                    meta: t.meta.and_then(|m| serde_json::from_str(&m).ok()),
                },
                ResourceContents::Blob(b) => rmcp::model::ResourceContents::BlobResourceContents {
                    uri: b.uri,
                    mime_type: b.mime_type,
                    blob: b.blob,
                    meta: b.meta.and_then(|m| serde_json::from_str(&m).ok()),
                },
            }
        }).collect(),
    };
    
    Ok(serde_json::to_value(result).unwrap())
}