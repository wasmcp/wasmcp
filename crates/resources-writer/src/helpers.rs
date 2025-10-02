//! Helper functions for JSON conversion and stream writing.

use crate::bindings::exports::wasmcp::mcp::resource_templates_list_result::Template;
use crate::bindings::exports::wasmcp::mcp::resources_list_result::Resource;
use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use serde_json::{json, Value};

/// Write data to the output stream, handling backpressure.
/// Does NOT flush - caller must flush when appropriate.
pub fn write_to_stream(output: &OutputStream, data: &[u8]) -> Result<(), StreamError> {
    let mut offset = 0;
    while offset < data.len() {
        let capacity = output.check_write().map_err(|_| StreamError::Closed)?;
        if capacity == 0 {
            // No capacity - must use blocking write for remainder
            output
                .blocking_write_and_flush(&data[offset..])
                .map_err(|_| StreamError::Closed)?;
            return Ok(()); // blocking_write_and_flush already flushed
        }

        let chunk_size = (capacity as usize).min(data.len() - offset);
        output
            .write(&data[offset..offset + chunk_size])
            .map_err(|_| StreamError::Closed)?;
        offset += chunk_size;
    }
    // Note: Caller must flush when appropriate
    Ok(())
}

/// Convert a Resource to JSON Value for serialization.
pub fn resource_to_json(resource: &Resource) -> Value {
    let mut result = json!({
        "uri": resource.uri,
        "name": resource.name
    });

    if let Some(options) = &resource.options {
        if let Some(size) = options.size {
            result["size"] = json!(size);
        }
        if let Some(title) = &options.title {
            result["title"] = json!(title);
        }
        if let Some(description) = &options.description {
            result["description"] = json!(description);
        }
        if let Some(mime_type) = &options.mime_type {
            result["mimeType"] = json!(mime_type);
        }
        // Note: annotations and meta could be added here if needed
    }

    result
}

/// Convert a Template to JSON Value for serialization.
pub fn resource_template_to_json(template: &Template) -> Value {
    let mut result = json!({
        "uriTemplate": template.uri_template,
        "name": template.name
    });

    if let Some(options) = &template.options {
        if let Some(description) = &options.description {
            result["description"] = json!(description);
        }
        if let Some(title) = &options.title {
            result["title"] = json!(title);
        }
        if let Some(mime_type) = &options.mime_type {
            result["mimeType"] = json!(mime_type);
        }
        // Note: annotations and meta could be added here if needed
    }

    result
}
