//! Content block serialization functions
//!
//! Handles conversion of MCP content blocks to JSON-serializable format,
//! including streaming data support for text and blobs.

use crate::bindings::wasmcp::mcp_v20250618::mcp::ContentBlock;
use crate::serializer::types::{
    convert_annotations, convert_blob_data, convert_text_data, JsonContentBlock, JsonImageContent,
    JsonResourceContent, JsonTextContent,
};

/// Convert ContentBlock with streaming support
///
/// This demonstrates the streaming infrastructure in action.
/// Handles text-stream and blob-stream variants with bounded memory.
pub fn convert_content_block(block: &ContentBlock) -> Result<JsonContentBlock, String> {
    match block {
        ContentBlock::Text(text_content) => {
            let text = convert_text_data(&text_content.text)?;
            Ok(JsonContentBlock::Text(JsonTextContent {
                text,
                annotations: text_content
                    .options
                    .as_ref()
                    .and_then(|o| o.annotations.as_ref())
                    .map(convert_annotations),
            }))
        }
        ContentBlock::Image(image_content) => {
            let data = convert_blob_data(&image_content.data)?;
            Ok(JsonContentBlock::Image(JsonImageContent {
                data,
                mime_type: image_content.mime_type.clone(),
                annotations: image_content
                    .options
                    .as_ref()
                    .and_then(|o| o.annotations.as_ref())
                    .map(convert_annotations),
            }))
        }
        ContentBlock::Audio(audio_content) => {
            let data = convert_blob_data(&audio_content.data)?;
            Ok(JsonContentBlock::Audio(JsonImageContent {
                data,
                mime_type: audio_content.mime_type.clone(),
                annotations: audio_content
                    .options
                    .as_ref()
                    .and_then(|o| o.annotations.as_ref())
                    .map(convert_annotations),
            }))
        }
        ContentBlock::ResourceLink(link) => Ok(JsonContentBlock::Resource(JsonResourceContent {
            uri: link.uri.clone(),
            text: None,
            blob: None,
            mime_type: link.options.as_ref().and_then(|o| o.mime_type.clone()),
        })),
        ContentBlock::EmbeddedResource(embedded) => {
            use crate::bindings::wasmcp::mcp_v20250618::mcp::ResourceContents;
            match &embedded.resource {
                ResourceContents::Text(text_res) => {
                    let text = convert_text_data(&text_res.text)?;
                    Ok(JsonContentBlock::Resource(JsonResourceContent {
                        uri: text_res.uri.clone(),
                        text: Some(text),
                        blob: None,
                        mime_type: text_res.options.as_ref().and_then(|o| o.mime_type.clone()),
                    }))
                }
                ResourceContents::Blob(blob_res) => {
                    let blob = convert_blob_data(&blob_res.blob)?;
                    Ok(JsonContentBlock::Resource(JsonResourceContent {
                        uri: blob_res.uri.clone(),
                        text: None,
                        blob: Some(blob),
                        mime_type: blob_res.options.as_ref().and_then(|o| o.mime_type.clone()),
                    }))
                }
            }
        }
    }
}
