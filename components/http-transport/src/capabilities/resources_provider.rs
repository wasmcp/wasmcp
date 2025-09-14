use crate::bindings;
use wasmcp_core::{
    McpResourcesHandler, McpError,
    ListResourcesRequest, ListResourcesResult,
    ReadResourceRequest, ReadResourceResult
};

/// A concrete implementation of the resources provider that communicates with the WASM host
/// through the generated WIT bindings.
pub struct ResourcesProvider;

impl McpResourcesHandler for ResourcesProvider {
    fn list_resources(&self, request: ListResourcesRequest) -> Result<ListResourcesResult, McpError> {
        // Convert wasmcp-core request to WIT types
        let wit_request = bindings::wasmcp::transport::resources::ListResourcesRequest {
            cursor: request.cursor,
        };

        let response = bindings::wasmcp::transport::resources::list_resources(&wit_request)?;

        // Convert WIT ListResourcesResult to wasmcp-core ListResourcesResult
        let result = ListResourcesResult {
            resources: response.resources.into_iter().map(|r| {
                wasmcp_core::McpResource {
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
                    annotations: r.annotations,
                }
            }).collect(),
            next_cursor: response.next_cursor,
        };

        Ok(result)
    }

    fn read_resource(&self, request: ReadResourceRequest) -> Result<ReadResourceResult, McpError> {
        // Convert wasmcp-core request to WIT types
        let wit_request = bindings::wasmcp::transport::resources::ReadResourceRequest {
            uri: request.uri,
        };

        let response = bindings::wasmcp::transport::resources::read_resource(&wit_request)?;

        // Convert WIT response to wasmcp-core result
        let result = ReadResourceResult {
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
        };

        Ok(result)
    }
}