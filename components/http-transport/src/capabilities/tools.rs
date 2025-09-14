use crate::auth::AuthContext;
use crate::bindings;
use crate::error::{ErrorCode, McpError};
use rmcp::model::{CallToolRequestParam, CallToolResult, ListToolsResult, PaginatedRequestParam};
use serde_json::Value;

pub fn list_tools(params: Option<Value>) -> Result<Value, McpError> {
    let _params: Option<PaginatedRequestParam> = params
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| McpError {
            code: ErrorCode::InvalidParams,
            message: format!("Invalid params: {e}"),
            data: None,
        })?;

    let request = bindings::wasmcp::transport::tools::ListToolsRequest {
        cursor: None,
    };

    let response = bindings::wasmcp::transport::tools::list_tools(&request)?;
    
    // Convert WIT ListToolsResult to rmcp ListToolsResult
    let result = ListToolsResult {
        tools: response.tools.into_iter().map(|t| {
            let input_schema = serde_json::from_str(&t.input_schema)
                .unwrap_or_else(|_| serde_json::Map::new());
            let output_schema = t.output_schema.and_then(|s| {
                serde_json::from_str(&s).ok()
            });
            
            rmcp::model::Tool {
                name: std::borrow::Cow::Owned(t.name),
                title: t.title,
                description: t.description.map(std::borrow::Cow::Owned),
                input_schema: std::sync::Arc::new(input_schema),
                output_schema: output_schema.map(std::sync::Arc::new),
                annotations: t.annotations.map(|a| rmcp::model::ToolAnnotations {
                    title: a.title,
                    read_only_hint: a.read_only_hint,
                    destructive_hint: a.destructive_hint,
                    idempotent_hint: a.idempotent_hint,
                    open_world_hint: a.open_world_hint,
                }),
                icons: t.icons.map(|icons| {
                    icons.into_iter().map(|i| rmcp::model::Icon {
                        src: i.src,
                        mime_type: i.mime_type,
                        sizes: i.sizes,
                    }).collect()
                }),
            }
        }).collect(),
        next_cursor: response.next_cursor,
    };
    
    Ok(serde_json::to_value(result).unwrap())
}

pub fn call_tool(params: Option<Value>, auth_context: Option<&AuthContext>) -> Result<Value, McpError> {
    let params: CallToolRequestParam = params
        .ok_or_else(|| McpError {
            code: ErrorCode::InvalidParams,
            message: "Missing params".to_string(),
            data: None,
        })
        .and_then(|p| {
            serde_json::from_value(p).map_err(|e| McpError {
                code: ErrorCode::InvalidParams,
                message: format!("Invalid params: {e}"),
                data: None,
            })
        })?;

    // Convert request to WIT types
    let request = bindings::wasmcp::transport::tools::CallToolRequest {
        name: params.name.to_string(),
        arguments: params.arguments.map(|args| {
            serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string())
        }),
    };

    // Call WIT binding directly with auth context if available (None if auth disabled)
    let response = bindings::wasmcp::transport::tools::call_tool(&request, auth_context)
        .map_err(|e| McpError {
            code: ErrorCode::InternalError,
            message: e.message,
            data: None,
        })?;

    // Convert WIT response to rmcp result
    let result = CallToolResult {
        content: response.content.into_iter().map(|c| {
            use wasmcp_core::wasmcp::mcp::mcp_types::ContentBlock;
            match c {
                ContentBlock::Text(t) => rmcp::model::Content::text(t.text),
                ContentBlock::Image(i) => rmcp::model::Content::image(i.data, i.mime_type),
                ContentBlock::Audio(a) => {
                    // rmcp doesn't have direct audio support, convert to text
                    rmcp::model::Content::text(format!("[Audio: {} - {} bytes]", a.mime_type, a.data.len()))
                },
                ContentBlock::Resource(r) => {
                    use wasmcp_core::wasmcp::mcp::mcp_types::ResourceContents;
                    let resource_contents = match r.resource {
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
                    };
                    rmcp::model::Content::resource(resource_contents)
                },
                ContentBlock::ResourceLink(r) => {
                    rmcp::model::Content::resource_link(rmcp::model::RawResource {
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
                    })
                },
            }
        }).collect(),
        structured_content: response.structured_content.and_then(|s| {
            serde_json::from_str(&s).ok()
        }),
        is_error: response.is_error,
        meta: response.meta.and_then(|m| serde_json::from_str(&m).ok()),
    };
    
    Ok(serde_json::to_value(result).unwrap())
}