use crate::bindings;
use crate::error::{ErrorCode, McpError};
use rmcp::model::{GetPromptResult, ListPromptsResult, PaginatedRequestParam, PromptMessage};
use serde_json::Value;

pub fn list_prompts(params: Option<Value>) -> Result<Value, McpError> {
    let _params: Option<PaginatedRequestParam> = params
        .map(|p| serde_json::from_value(p))
        .transpose()
        .map_err(|e| McpError {
            code: ErrorCode::InvalidParams,
            message: format!("Invalid params: {}", e),
            data: None,
        })?;

    let request = bindings::wasmcp::transport::prompts_types::ListPromptsRequest {
        cursor: None,
    };

    let response = bindings::wasmcp::transport::prompts::list_prompts(&request)?;
    
    // Convert WIT ListPromptsResult to rmcp
    let result = ListPromptsResult {
        prompts: response.prompts.into_iter().map(|p| {
            rmcp::model::Prompt {
                name: p.name,
                title: p.title,
                description: p.description,
                arguments: p.arguments.map(|args| {
                    args.into_iter().map(|a| rmcp::model::PromptArgument {
                        name: a.name,
                        title: a.title,
                        description: a.description,
                        required: a.required,
                    }).collect()
                }),
                icons: p.icons.map(|icons| {
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

pub fn get_prompt(params: Option<Value>) -> Result<Value, McpError> {
    let params = params.ok_or_else(|| McpError {
        code: ErrorCode::InvalidParams,
        message: "Missing params".to_string(),
        data: None,
    })?;

    let name = params["name"].as_str().ok_or_else(|| McpError {
        code: ErrorCode::InvalidParams,
        message: "Missing name parameter".to_string(),
        data: None,
    })?;

    let arguments = params
        .get("arguments")
        .and_then(|v| v.as_object())
        .map(|obj| serde_json::to_string(obj).unwrap_or_else(|_| "{}".to_string()));

    let request = bindings::wasmcp::transport::prompts_types::GetPromptRequest {
        name: name.to_string(),
        arguments,
    };

    let response = bindings::wasmcp::transport::prompts::get_prompt(&request)?;
    
    // Convert WIT GetPromptResult to rmcp
    let result = GetPromptResult {
        description: response.description,
        messages: response.messages.into_iter().map(|m| {
            convert_prompt_message(m)
        }).collect(),
    };
    
    Ok(serde_json::to_value(result).unwrap())
}

/// Helper function to convert WIT PromptMessage to rmcp PromptMessage
fn convert_prompt_message(m: bindings::wasmcp::transport::prompts_types::PromptMessage) -> PromptMessage {
    use bindings::wasmcp::transport::prompts_types::{PromptMessageContent, PromptMessageRole};
    
    PromptMessage {
        role: match m.role {
            PromptMessageRole::User => rmcp::model::PromptMessageRole::User,
            PromptMessageRole::Assistant => rmcp::model::PromptMessageRole::Assistant,
        },
        content: match m.content {
            PromptMessageContent::Text(t) => {
                rmcp::model::PromptMessageContent::Text { text: t.text }
            },
            PromptMessageContent::Image(i) => {
                rmcp::model::PromptMessageContent::Image {
                    image: rmcp::model::ImageContent::new(
                        rmcp::model::RawImageContent {
                            data: i.data,
                            mime_type: i.mime_type,
                            meta: i.meta.and_then(|m| serde_json::from_str(&m).ok()),
                        },
                        i.annotations.map(|a| convert_annotations(a)),
                    ),
                }
            },
            PromptMessageContent::Resource(r) => {
                rmcp::model::PromptMessageContent::Resource {
                    resource: rmcp::model::EmbeddedResource::new(
                        rmcp::model::RawEmbeddedResource {
                            meta: r.meta.and_then(|m| serde_json::from_str(&m).ok()),
                            resource: convert_resource_contents(r.resource),
                        },
                        r.annotations.map(|a| convert_annotations(a)),
                    ),
                }
            },
            PromptMessageContent::ResourceLink(r) => {
                rmcp::model::PromptMessageContent::ResourceLink {
                    link: rmcp::model::Resource::new(
                        rmcp::model::RawResource {
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
                        },
                        r.annotations.map(|a| convert_annotations(a)),
                    ),
                }
            },
        },
    }
}

/// Helper function to convert WIT Annotations to rmcp Annotations
fn convert_annotations(a: bindings::wasmcp::mcp::mcp_types::Annotations) -> rmcp::model::Annotations {
    rmcp::model::Annotations {
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
    }
}

/// Helper function to convert WIT ResourceContents to rmcp ResourceContents
fn convert_resource_contents(
    contents: bindings::wasmcp::mcp::mcp_types::ResourceContents
) -> rmcp::model::ResourceContents {
    use bindings::wasmcp::mcp::mcp_types::ResourceContents;
    
    match contents {
        ResourceContents::Text(t) => {
            rmcp::model::ResourceContents::TextResourceContents {
                uri: t.uri,
                mime_type: t.mime_type,
                text: t.text,
                meta: t.meta.and_then(|m| serde_json::from_str(&m).ok()),
            }
        },
        ResourceContents::Blob(b) => {
            rmcp::model::ResourceContents::BlobResourceContents {
                uri: b.uri,
                mime_type: b.mime_type,
                blob: b.blob,
                meta: b.meta.and_then(|m| serde_json::from_str(&m).ok()),
            }
        },
    }
}