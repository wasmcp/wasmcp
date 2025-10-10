//! Resources response writers
//!
//! Implements serialization for resource-related MCP responses including:
//! - Resource listings with metadata and pagination
//! - Resource content reading with streaming support
//! - Resource template listings for dynamic resources

use crate::bindings::exports::wasmcp::mcp::resources_response;
use crate::bindings::wasmcp::mcp::output::{
    finish_message, start_message, write_message_contents, IoError,
};
use crate::bindings::wasmcp::mcp::protocol::{
    Id, MetaOptions, NextCursorOptions, Resource, ResourceContents, ResourceTemplate,
};
use crate::utils::{base64_encode, escape_json_string, JsonObjectBuilder};
use std::cell::RefCell;

// === Simple Response Functions ===

/// Write a resources/list response with complete resource list.
pub fn write_resources(id: Id, resources: Vec<Resource>) -> Result<(), IoError> {
    let resources_json = build_resources_array(&resources);

    let mut result = JsonObjectBuilder::new();
    result.add_raw_json("resources", &resources_json);

    let response = build_json_rpc_response(&id, &result.build());

    start_message()?;
    write_message_contents(&response.into_bytes())?;
    finish_message()
}

/// Write a resources/read response with resource contents.
pub fn write_contents(id: Id, contents: ResourceContents) -> Result<(), IoError> {
    let mut result = JsonObjectBuilder::new();
    result.add_string("uri", &contents.uri);

    // Check if data is valid UTF-8 text or binary
    if let Ok(text) = String::from_utf8(contents.data.clone()) {
        result.add_string("text", &text);
    } else {
        result.add_string("blob", &base64_encode(&contents.data));
    }

    // Add optional mime type from options
    if let Some(opts) = &contents.options {
        if let Some(mime) = &opts.mime_type {
            result.add_string("mimeType", mime);
        }
    }

    let response = build_json_rpc_response(&id, &result.build());

    start_message()?;
    write_message_contents(&response.into_bytes())?;
    finish_message()
}

/// Write a resources/templates/list response with complete template list.
pub fn write_templates(id: Id, templates: Vec<ResourceTemplate>) -> Result<(), IoError> {
    let templates_json = build_templates_array(&templates);

    let mut result = JsonObjectBuilder::new();
    result.add_raw_json("resourceTemplates", &templates_json);

    let response = build_json_rpc_response(&id, &result.build());

    start_message()?;
    write_message_contents(&response.into_bytes())?;
    finish_message()
}

// === Streaming Resources Writer Resource ===

pub struct ResourcesWriter {
    state: RefCell<ResourcesWriterState>,
}

struct ResourcesWriterState {
    first_item: bool,
}

impl crate::bindings::exports::wasmcp::mcp::resources_response::GuestResourcesWriter
    for ResourcesWriter
{
    fn start(id: Id) -> Result<resources_response::ResourcesWriter, IoError> {
        let id_str = format_id(&id);
        let header = format!(r#"{{"jsonrpc":"2.0","id":{id_str},"result":{{"resources":["#);

        start_message()?;
        write_message_contents(&header.into_bytes())?;

        Ok(resources_response::ResourcesWriter::new(ResourcesWriter {
            state: RefCell::new(ResourcesWriterState { first_item: true }),
        }))
    }

    fn write(&self, resource: Resource) -> Result<(), IoError> {
        let mut state = self.state.borrow_mut();

        let mut output_str = String::new();
        if !state.first_item {
            output_str.push(',');
        } else {
            state.first_item = false;
        }

        let resource_json = build_single_resource(&resource);
        output_str.push_str(&resource_json);

        write_message_contents(&output_str.into_bytes())
    }

    fn finish(
        _this: resources_response::ResourcesWriter,
        options: Option<NextCursorOptions>,
    ) -> Result<(), IoError> {
        let mut closing = String::from("]");

        if let Some(opts) = options {
            if let Some(cursor) = opts.next_cursor {
                closing.push_str(&format!(
                    r#","nextCursor":"{}""#,
                    escape_json_string(&cursor)
                ));
            }
            if let Some(meta) = opts.meta {
                if !meta.is_empty() {
                    let meta_json = build_meta_json(&meta);
                    closing.push_str(&format!(r#","_meta":{}"#, meta_json));
                }
            }
        }

        closing.push_str("}}");

        write_message_contents(&closing.into_bytes())?;
        finish_message()
    }
}

// === Streaming Contents Writer Resource ===

pub struct ContentsWriter {
    state: RefCell<ContentsWriterState>,
}

struct ContentsWriterState {
    id: Id,
    uri: String,
    chunks: Vec<Vec<u8>>,
    mime_type: Option<String>,
}

impl crate::bindings::exports::wasmcp::mcp::resources_response::GuestContentsWriter
    for ContentsWriter
{
    fn start(
        id: Id,
        initial: ResourceContents,
    ) -> Result<resources_response::ContentsWriter, IoError> {
        // For streaming resource contents, we need to buffer the data
        // because we can't determine if it's text or binary until we have all chunks
        let initial_data = initial.data.clone();

        let mime_type = initial
            .options
            .as_ref()
            .and_then(|opts| opts.mime_type.clone());

        Ok(resources_response::ContentsWriter::new(ContentsWriter {
            state: RefCell::new(ContentsWriterState {
                id,
                uri: initial.uri,
                chunks: vec![initial_data],
                mime_type,
            }),
        }))
    }

    fn write(&self, contents: Vec<u8>) -> Result<(), IoError> {
        let mut state = self.state.borrow_mut();
        state.chunks.push(contents);
        Ok(())
    }

    fn finish(
        this: resources_response::ContentsWriter,
        options: Option<MetaOptions>,
    ) -> Result<(), IoError> {
        // Get inner implementation from wrapper
        let inner: ContentsWriter = this.into_inner();
        let state = inner.state.borrow();

        let mut result = JsonObjectBuilder::new();
        result.add_string("uri", &state.uri);

        // Combine all chunks
        let total_len: usize = state.chunks.iter().map(|c| c.len()).sum();
        let mut combined = Vec::with_capacity(total_len);
        for chunk in &state.chunks {
            combined.extend_from_slice(chunk);
        }

        // Check if it's valid UTF-8 text
        if let Ok(text) = String::from_utf8(combined.clone()) {
            result.add_string("text", &text);
        } else {
            // Binary data - encode as base64
            result.add_string("blob", &base64_encode(&combined));
        }

        // Add mime type
        if let Some(mime) = &state.mime_type {
            result.add_string("mimeType", mime);
        }

        // Add meta from options
        if let Some(opts) = options {
            if let Some(meta) = opts.meta {
                if !meta.is_empty() {
                    result.add_raw_json("_meta", &build_meta_json(&meta));
                }
            }
        }

        let response = build_json_rpc_response(&state.id, &result.build());

        start_message()?;
        write_message_contents(&response.into_bytes())?;
        finish_message()
    }
}

// === Streaming Templates Writer Resource ===

pub struct TemplatesWriter {
    state: RefCell<TemplatesWriterState>,
}

struct TemplatesWriterState {
    first_item: bool,
}

impl crate::bindings::exports::wasmcp::mcp::resources_response::GuestTemplatesWriter
    for TemplatesWriter
{
    fn start(id: Id) -> Result<resources_response::TemplatesWriter, IoError> {
        let id_str = format_id(&id);
        let header = format!(r#"{{"jsonrpc":"2.0","id":{id_str},"result":{{"resourceTemplates":["#);

        start_message()?;
        write_message_contents(&header.into_bytes())?;

        Ok(resources_response::TemplatesWriter::new(TemplatesWriter {
            state: RefCell::new(TemplatesWriterState { first_item: true }),
        }))
    }

    fn write(&self, template: ResourceTemplate) -> Result<(), IoError> {
        let mut state = self.state.borrow_mut();

        let mut output_str = String::new();
        if !state.first_item {
            output_str.push(',');
        } else {
            state.first_item = false;
        }

        let template_json = build_single_template(&template);
        output_str.push_str(&template_json);

        write_message_contents(&output_str.into_bytes())
    }

    fn finish(
        _this: resources_response::TemplatesWriter,
        options: Option<NextCursorOptions>,
    ) -> Result<(), IoError> {
        let mut closing = String::from("]");

        if let Some(opts) = options {
            if let Some(cursor) = opts.next_cursor {
                closing.push_str(&format!(
                    r#","nextCursor":"{}""#,
                    escape_json_string(&cursor)
                ));
            }
            if let Some(meta) = opts.meta {
                if !meta.is_empty() {
                    let meta_json = build_meta_json(&meta);
                    closing.push_str(&format!(r#","_meta":{}"#, meta_json));
                }
            }
        }

        closing.push_str("}}");

        write_message_contents(&closing.into_bytes())?;
        finish_message()
    }
}

// === Helper Functions ===

/// Build a JSON array of resources.
fn build_resources_array(resources: &[Resource]) -> String {
    if resources.is_empty() {
        return "[]".to_string();
    }

    let resource_jsons: Vec<String> = resources.iter().map(build_single_resource).collect();
    format!("[{}]", resource_jsons.join(","))
}

/// Build JSON for a single resource.
fn build_single_resource(resource: &Resource) -> String {
    let mut obj = JsonObjectBuilder::new();
    obj.add_string("uri", &resource.uri);
    obj.add_string("name", &resource.name);

    if let Some(opts) = &resource.options {
        if let Some(description) = &opts.description {
            obj.add_string("description", description);
        }
        if let Some(mime_type) = &opts.mime_type {
            obj.add_string("mimeType", mime_type);
        }
        if let Some(annotations) = &opts.annotations {
            obj.add_raw_json("annotations", &build_annotations_json(annotations));
        }
        if let Some(meta) = &opts.meta {
            if !meta.is_empty() {
                obj.add_raw_json("_meta", &build_meta_json(meta));
            }
        }
    }

    obj.build()
}

/// Build a JSON array of resource templates.
fn build_templates_array(templates: &[ResourceTemplate]) -> String {
    if templates.is_empty() {
        return "[]".to_string();
    }

    let template_jsons: Vec<String> = templates.iter().map(build_single_template).collect();
    format!("[{}]", template_jsons.join(","))
}

/// Build JSON for a single resource template.
fn build_single_template(template: &ResourceTemplate) -> String {
    let mut obj = JsonObjectBuilder::new();
    obj.add_string("uriTemplate", &template.uri_template);
    obj.add_string("name", &template.name);

    if let Some(opts) = &template.options {
        if let Some(description) = &opts.description {
            obj.add_string("description", description);
        }
        if let Some(mime_type) = &opts.mime_type {
            obj.add_string("mimeType", mime_type);
        }
        if let Some(annotations) = &opts.annotations {
            obj.add_raw_json("annotations", &build_annotations_json(annotations));
        }
        if let Some(meta) = &opts.meta {
            if !meta.is_empty() {
                obj.add_raw_json("_meta", &build_meta_json(meta));
            }
        }
    }

    obj.build()
}

/// Build JSON for resource annotations.
fn build_annotations_json(ann: &crate::bindings::wasmcp::mcp::protocol::Annotations) -> String {
    let mut obj = JsonObjectBuilder::new();

    if let Some(audience) = &ann.audience {
        let audiences: Vec<String> = audience
            .iter()
            .map(|a| {
                let role_str = match a {
                    crate::bindings::wasmcp::mcp::protocol::Role::User => "user",
                    crate::bindings::wasmcp::mcp::protocol::Role::Assistant => "assistant",
                };
                format!(r#""{}""#, role_str)
            })
            .collect();
        obj.add_raw_json("audience", &format!("[{}]", audiences.join(",")));
    }
    // priority is f64, not Option<f64>
    obj.add_number("priority", ann.priority);

    obj.build()
}

/// Build JSON for metadata.
fn build_meta_json(meta: &[(String, String)]) -> String {
    let mut obj = JsonObjectBuilder::new();
    for (key, value) in meta {
        obj.add_string(key, value);
    }
    obj.build()
}

/// Build a JSON-RPC 2.0 response.
fn build_json_rpc_response(id: &Id, result: &str) -> String {
    let id_str = format_id(id);
    format!(r#"{{"jsonrpc":"2.0","id":{},"result":{}}}"#, id_str, result)
}

/// Format an ID value as JSON.
fn format_id(id: &Id) -> String {
    match id {
        Id::Number(n) => n.to_string(),
        Id::String(s) => format!(r#""{}""#, escape_json_string(s)),
    }
}
