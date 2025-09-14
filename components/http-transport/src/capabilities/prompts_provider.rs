use crate::bindings;
use wasmcp_core::{
    McpPromptsHandler, McpError,
    ListPromptsRequest, ListPromptsResult,
    GetPromptRequest, GetPromptResult
};

/// A concrete implementation of the prompts provider that communicates with the WASM host
/// through the generated WIT bindings.
pub struct PromptsProvider;

impl McpPromptsHandler for PromptsProvider {
    fn list_prompts(&self, request: ListPromptsRequest) -> Result<ListPromptsResult, McpError> {
        // Convert wasmcp-core request to WIT types
        let wit_request = bindings::wasmcp::transport::prompts::ListPromptsRequest {
            cursor: request.cursor,
        };

        let response = bindings::wasmcp::transport::prompts::list_prompts(&wit_request)?;

        // Convert WIT ListPromptsResult to wasmcp-core ListPromptsResult
        let result = ListPromptsResult {
            prompts: response.prompts.into_iter().map(|p| {
                wasmcp_core::Prompt {
                    name: p.name,
                    title: p.title,
                    description: p.description,
                    arguments: p.arguments.map(|args| {
                        args.into_iter().map(|a| wasmcp_core::PromptArgument {
                            name: a.name,
                            title: a.title,
                            description: a.description,
                            required: a.required,
                        }).collect()
                    }),
                    icons: p.icons.map(|icons| {
                        icons.into_iter().map(|i| wasmcp_core::Icon {
                            src: i.src,
                            mime_type: i.mime_type,
                            sizes: i.sizes,
                        }).collect()
                    }),
                }
            }).collect(),
            next_cursor: response.next_cursor,
        };

        Ok(result)
    }

    fn get_prompt(&self, request: GetPromptRequest) -> Result<GetPromptResult, McpError> {
        // Convert wasmcp-core request to WIT types
        let wit_request = bindings::wasmcp::transport::prompts::GetPromptRequest {
            name: request.name,
            arguments: request.arguments,
        };

        let response = bindings::wasmcp::transport::prompts::get_prompt(&wit_request)?;

        // Convert WIT response to wasmcp-core result
        let result = GetPromptResult {
            description: response.description,
            messages: response.messages.into_iter().map(|m| {
                wasmcp_core::PromptMessage {
                    role: match m.role {
                        bindings::wasmcp::transport::prompts::PromptMessageRole::User => wasmcp_core::PromptMessageRole::User,
                        bindings::wasmcp::transport::prompts::PromptMessageRole::Assistant => wasmcp_core::PromptMessageRole::Assistant,
                    },
                    content: match m.content {
                        bindings::wasmcp::transport::prompts::PromptMessageContent::Text(t) => {
                            wasmcp_core::PromptMessageContent::Text(wasmcp_core::TextContent {
                                text: t.text,
                                meta: t.meta,
                                annotations: t.annotations,
                            })
                        },
                        bindings::wasmcp::transport::prompts::PromptMessageContent::Image(i) => {
                            wasmcp_core::PromptMessageContent::Image(wasmcp_core::ImageContent {
                                data: i.data,
                                mime_type: i.mime_type,
                                meta: i.meta,
                                annotations: i.annotations,
                            })
                        },
                        bindings::wasmcp::transport::prompts::PromptMessageContent::Resource(r) => {
                            use wasmcp_core::wasmcp::mcp::mcp_types::ResourceContents;
                            let resource_contents = match r.resource {
                                ResourceContents::Text(t) => wasmcp_core::ResourceContents::Text(wasmcp_core::TextResourceContents {
                                    uri: t.uri,
                                    mime_type: t.mime_type,
                                    text: t.text,
                                    meta: t.meta,
                                }),
                                ResourceContents::Blob(b) => wasmcp_core::ResourceContents::Blob(wasmcp_core::BlobResourceContents {
                                    uri: b.uri,
                                    mime_type: b.mime_type,
                                    blob: b.blob,
                                    meta: b.meta,
                                }),
                            };
                            wasmcp_core::PromptMessageContent::Resource(wasmcp_core::EmbeddedResource {
                                meta: r.meta,
                                resource: resource_contents,
                                annotations: r.annotations,
                            })
                        },
                        bindings::wasmcp::transport::prompts::PromptMessageContent::ResourceLink(r) => {
                            wasmcp_core::PromptMessageContent::ResourceLink(wasmcp_core::RawResource {
                                uri: r.uri,
                                name: r.name,
                                title: r.title,
                                description: r.description,
                                mime_type: r.mime_type,
                                size: r.size,
                                icons: r.icons.map(|icons| {
                                    icons.into_iter().map(|i| wasmcp_core::Icon {
                                        src: i.src,
                                        mime_type: i.mime_type,
                                        sizes: i.sizes,
                                    }).collect()
                                }),
                            })
                        },
                    },
                }
            }).collect(),
        };

        Ok(result)
    }
}