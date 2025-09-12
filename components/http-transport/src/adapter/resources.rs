use super::WitMcpAdapter;
use anyhow::Result;
use base64::Engine;
use rmcp::model::{AnnotateAble, ListResourcesResult, RawResource, ReadResourceResult, ResourceContents};

impl WitMcpAdapter {
    /// Convert WIT ListResourcesResponse to rmcp ListResourcesResult
    pub fn convert_list_resources_to_rmcp(
        &self,
        response: crate::bindings::wasmcp::mcp::resources_types::ListResourcesResult,
    ) -> Result<ListResourcesResult> {
        let resources = response
            .resources
            .into_iter()
            .map(|r| {
                RawResource {
                    uri: r.uri.clone(),
                    name: r.base.title.unwrap_or(r.base.name),
                    description: r.description,
                    mime_type: r.mime_type,
                    size: None,
                }
                .no_annotation()
            })
            .collect();

        Ok(ListResourcesResult {
            resources,
            next_cursor: response.next_cursor,
        })
    }

    /// Convert WIT ReadResourceResponse to rmcp ReadResourceResult
    pub fn convert_read_resource_to_rmcp(
        &self,
        response: crate::bindings::wasmcp::mcp::resources_types::ReadResourceResult,
    ) -> Result<ReadResourceResult> {
        use crate::bindings::wasmcp::mcp::mcp_types::ResourceContents as WitResourceContents;

        let contents = response
            .contents
            .into_iter()
            .map(|c| match c {
                WitResourceContents::Text(text) => ResourceContents::TextResourceContents {
                    uri: text.uri,
                    mime_type: text.mime_type,
                    text: text.text,
                    meta: None,
                },
                WitResourceContents::Blob(blob) => ResourceContents::BlobResourceContents {
                    uri: blob.uri,
                    mime_type: blob.mime_type,
                    blob: base64::engine::general_purpose::STANDARD.encode(&blob.blob),
                    meta: None,
                },
            })
            .collect();

        Ok(ReadResourceResult { contents })
    }
}