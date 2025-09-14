use crate::bindings;
use wasmcp_core::{
    McpToolsHandler, McpError, AuthContext,
    ListToolsRequest, ListToolsResult, CallToolRequest, CallToolResult
};

/// A concrete implementation of the tools provider that communicates with the WASM host
/// through the generated WIT bindings.
pub struct ToolsProvider;

impl McpToolsHandler for ToolsProvider {
    fn list_tools(&self, request: ListToolsRequest) -> Result<ListToolsResult, McpError> {
        // Convert wasmcp-core request to WIT types
        let wit_request = bindings::wasmcp::transport::tools::ListToolsRequest {
            cursor: request.cursor,
        };

        let response = bindings::wasmcp::transport::tools::list_tools(&wit_request)?;

        // Convert WIT ListToolsResult to wasmcp-core ListToolsResult
        let result = ListToolsResult {
            tools: response.tools.into_iter().map(|t| {
                let input_schema = serde_json::from_str(&t.input_schema)
                    .unwrap_or_else(|_| serde_json::Map::new());
                let output_schema = t.output_schema.and_then(|s| {
                    serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&s).ok()
                });

                wasmcp_core::Tool {
                    name: t.name,
                    title: t.title,
                    description: t.description,
                    input_schema: serde_json::to_string(&input_schema).unwrap_or_else(|_| "{}".to_string()),
                    output_schema: output_schema.map(|s| serde_json::to_string(&s).unwrap_or_else(|_| "{}".to_string())),
                    annotations: t.annotations.map(|a| wasmcp_core::ToolAnnotations {
                        title: a.title,
                        read_only_hint: a.read_only_hint,
                        destructive_hint: a.destructive_hint,
                        idempotent_hint: a.idempotent_hint,
                        open_world_hint: a.open_world_hint,
                    }),
                    icons: t.icons.map(|icons| {
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

    fn call_tool(&self, request: CallToolRequest, context: Option<AuthContext>) -> Result<CallToolResult, McpError> {
        // Convert wasmcp-core request to WIT types
        let wit_request = bindings::wasmcp::transport::tools::CallToolRequest {
            name: request.name,
            arguments: request.arguments,
        };

        // Call WIT binding with auth context
        let response = bindings::wasmcp::transport::tools::call_tool(&wit_request, context.as_ref())
            .map_err(|e| McpError {
                code: wasmcp_core::ErrorCode::InternalError,
                message: e.message,
                data: None,
            })?;

        // Convert WIT response to wasmcp-core result
        let result = CallToolResult {
            content: response.content.into_iter().map(|c| {
                use wasmcp_core::wasmcp::mcp::mcp_types::ContentBlock;
                match c {
                    ContentBlock::Text(t) => wasmcp_core::ContentBlock::Text(wasmcp_core::TextContent {
                        text: t.text,
                        meta: t.meta,
                        annotations: t.annotations,
                    }),
                    ContentBlock::Image(i) => wasmcp_core::ContentBlock::Image(wasmcp_core::ImageContent {
                        data: i.data,
                        mime_type: i.mime_type,
                        meta: i.meta,
                        annotations: i.annotations,
                    }),
                    ContentBlock::Audio(a) => wasmcp_core::ContentBlock::Audio(wasmcp_core::AudioContent {
                        data: a.data,
                        mime_type: a.mime_type,
                        annotations: a.annotations,
                    }),
                    ContentBlock::Resource(r) => {
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
                        wasmcp_core::ContentBlock::Resource(wasmcp_core::EmbeddedResource {
                            meta: r.meta,
                            resource: resource_contents,
                            annotations: r.annotations,
                        })
                    },
                    ContentBlock::ResourceLink(r) => wasmcp_core::ContentBlock::ResourceLink(wasmcp_core::RawResource {
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
                    }),
                }
            }).collect(),
            structured_content: response.structured_content,
            is_error: response.is_error,
            meta: response.meta,
        };

        Ok(result)
    }
}