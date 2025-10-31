//! Content block parsing for MCP protocol
//!
//! This module handles parsing of content blocks from JSON into WIT types.

use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    Annotations, Blob, BlobData, ContentBlock, ContentOptions, Role, TextContent, TextData,
};
use serde_json::Value;

/// Parse a ContentBlock from sampling response JSON
///
/// Sampling responses contain LLM-generated content (text, image, audio)
/// and do not include streams or resource references.
///
/// Runtime constraints enforced:
/// - Only handles inline text/image/audio
/// - Streams: Not supported (LLMs return complete content; streaming is protocol-level)
/// - Resource links: Not supported (LLMs generate content, not references)
///
/// See: `sampling-create-message-result.content` in mcp.wit for rationale
pub(crate) fn parse_content_block(content: &Value) -> Result<ContentBlock, String> {
    let content_type = content
        .get("type")
        .and_then(|t| t.as_str())
        .ok_or("Missing 'type' in content block")?;

    match content_type {
        "text" => {
            // Defensive: Check for textStream (not supported in sampling)
            if content.get("textStream").is_some() {
                return Err(
                    "Text streams not supported in sampling responses. \
                     Sampling protocol handles streaming at the message level, not within content blocks."
                        .to_string(),
                );
            }

            let text = content
                .get("text")
                .and_then(|t| t.as_str())
                .ok_or("Missing 'text' field in text content block")?
                .to_string();

            let options = parse_content_options(content)?;

            Ok(ContentBlock::Text(TextContent {
                text: TextData::Text(text),
                options,
            }))
        }
        "image" => {
            // Defensive: Check for blobStream (not supported in sampling)
            if content.get("blobStream").is_some() {
                return Err("Image streams not supported in sampling responses. \
                     LLMs return complete generated images, not streams."
                    .to_string());
            }

            let data_b64 = content
                .get("data")
                .and_then(|d| d.as_str())
                .ok_or("Missing 'data' field in image content block")?;

            let mime_type = content
                .get("mimeType")
                .and_then(|m| m.as_str())
                .ok_or("Missing 'mimeType' field in image content block")?
                .to_string();

            // Decode base64 data
            let data = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data_b64)
                .map_err(|e| format!("Invalid base64 in image data: {}", e))?;

            let options = parse_content_options(content)?;

            Ok(ContentBlock::Image(Blob {
                data: BlobData::Blob(data),
                mime_type,
                options,
            }))
        }
        "audio" => {
            // Defensive: Check for blobStream (not supported in sampling)
            if content.get("blobStream").is_some() {
                return Err("Audio streams not supported in sampling responses. \
                     LLMs return complete generated audio, not streams."
                    .to_string());
            }

            let data_b64 = content
                .get("data")
                .and_then(|d| d.as_str())
                .ok_or("Missing 'data' field in audio content block")?;

            let mime_type = content
                .get("mimeType")
                .and_then(|m| m.as_str())
                .ok_or("Missing 'mimeType' field in audio content block")?
                .to_string();

            // Decode base64 data
            let data = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data_b64)
                .map_err(|e| format!("Invalid base64 in audio data: {}", e))?;

            let options = parse_content_options(content)?;

            Ok(ContentBlock::Audio(Blob {
                data: BlobData::Blob(data),
                mime_type,
                options,
            }))
        }
        "resource" => Err(
            "Resource content blocks not expected in sampling responses. \
             LLMs generate new content, not resource references. \
             Resource links are for prompt messages sent to LLMs, not sampling results."
                .to_string(),
        ),
        _ => Err(format!(
            "Unsupported content block type for sampling: '{}'. \
             Expected 'text', 'image', or 'audio'.",
            content_type
        )),
    }
}

/// Parse optional content-options from a content block JSON object
pub(crate) fn parse_content_options(content: &Value) -> Result<Option<ContentOptions>, String> {
    let meta = content
        .get("_meta")
        .and_then(|m| serde_json::to_string(m).ok());

    let annotations = content
        .get("annotations")
        .map(parse_annotations)
        .transpose()?;

    if meta.is_some() || annotations.is_some() {
        Ok(Some(ContentOptions { annotations, meta }))
    } else {
        Ok(None)
    }
}

/// Parse annotations from JSON
pub(crate) fn parse_annotations(annot: &Value) -> Result<Annotations, String> {
    let audience = annot
        .get("audience")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    v.as_str().and_then(|s| match s {
                        "user" => Some(Role::User),
                        "assistant" => Some(Role::Assistant),
                        _ => None,
                    })
                })
                .collect::<Vec<_>>()
        })
        .filter(|v| !v.is_empty());

    let last_modified = annot
        .get("lastModified")
        .and_then(|m| m.as_str())
        .map(String::from);

    let priority = annot.get("priority").and_then(|p| p.as_f64());

    Ok(Annotations {
        audience,
        last_modified,
        priority,
    })
}