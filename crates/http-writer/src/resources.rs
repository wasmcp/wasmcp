//! Resources writer implementations for HTTP/SSE transport.
//!
//! Handles serialization of resource-related responses including:
//! - Resource listings with metadata and pagination
//! - Resource content reading with streaming support
//! - Resource template listings for dynamic resources

use crate::bindings::exports::wasmcp::mcp::{
    list_resources_writer, read_resource_writer, list_resource_templates_writer,
};
use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use crate::bindings::wasmcp::mcp::protocol::Id;
use crate::utils::{
    base64_encode, build_annotations_json, build_jsonrpc_response,
    build_meta_object, write_sse_message, JsonObjectBuilder,
};
use std::cell::RefCell;

pub struct ListResourcesWriter;
pub struct ReadResourceWriter;
pub struct ListResourceTemplatesWriter;

// ===== List Resources Writer =====

impl list_resources_writer::Guest for ListResourcesWriter {
    fn send(
        id: Id,
        out: OutputStream,
        resources: Vec<list_resources_writer::Resource>,
    ) -> Result<(), StreamError> {
        let resources_json = build_resources_array(&resources);

        let mut result = JsonObjectBuilder::new();
        result.add_field("resources", &resources_json);

        let response = build_jsonrpc_response(&id, &result.build());
        write_sse_message(&out, &response)
    }

    type Writer = ListResourcesWriterResource;

    fn open(
        id: Id,
        out: OutputStream,
    ) -> Result<list_resources_writer::Writer, StreamError> {
        Ok(list_resources_writer::Writer::new(ListResourcesWriterResource {
            state: RefCell::new(ResourcesWriterState {
                id,
                out,
                resources: Vec::new(),
            }),
        }))
    }
}

pub struct ListResourcesWriterResource {
    state: RefCell<ResourcesWriterState>,
}

struct ResourcesWriterState {
    id: Id,
    out: OutputStream,
    resources: Vec<list_resources_writer::Resource>,
}

impl list_resources_writer::GuestWriter for ListResourcesWriterResource {
    fn write(&self, resource: list_resources_writer::Resource) -> Result<(), StreamError> {
        let mut state = self.state.borrow_mut();
        state.resources.push(resource);
        Ok(())
    }

    fn close(&self, options: Option<list_resources_writer::Options>) -> Result<(), StreamError> {
        let state = self.state.borrow();
        let resources_json = build_resources_array(&state.resources);

        let mut result = JsonObjectBuilder::new();
        result.add_field("resources", &resources_json);

        if let Some(opts) = options {
            if let Some(meta) = opts.meta {
                if !meta.is_empty() {
                    result.add_field("_meta", &build_meta_object(&meta));
                }
            }
            if let Some(Some(cursor)) = opts.next_cursor.as_ref() {
                result.add_string("nextCursor", cursor);
            }
        }

        let response = build_jsonrpc_response(&state.id, &result.build());
        write_sse_message(&state.out, &response)
    }
}

// ===== Read Resource Writer =====

impl read_resource_writer::Guest for ReadResourceWriter {
    fn send(
        id: Id,
        out: OutputStream,
        contents: read_resource_writer::ResourceContents,
    ) -> Result<(), StreamError> {
        let mut result = JsonObjectBuilder::new();
        result.add_string("uri", &contents.uri);

        // Handle the data - check if it's text or binary
        if let Ok(text) = String::from_utf8(contents.data.clone()) {
            result.add_string("text", &text);
        } else {
            // Base64 encode binary data
            result.add_string("blob", &base64_encode(&contents.data));
        }

        // Add optional fields
        if let Some(opts) = contents.options {
            if let Some(mime) = opts.mime_type {
                result.add_string("mimeType", &mime);
            }
            if let Some(meta) = opts.meta {
                if !meta.is_empty() {
                    result.add_field("_meta", &build_meta_object(&meta));
                }
            }
        }

        let response = build_jsonrpc_response(&id, &result.build());
        write_sse_message(&out, &response)
    }

    type Writer = ReadResourceWriterResource;

    fn open(
        id: Id,
        out: OutputStream,
        initial: read_resource_writer::ResourceContents,
    ) -> Result<read_resource_writer::Writer, StreamError> {
        Ok(read_resource_writer::Writer::new(ReadResourceWriterResource {
            state: RefCell::new(ReadResourceWriterState {
                id,
                out,
                uri: initial.uri,
                chunks: vec![initial.data],
                options: initial.options,
            }),
        }))
    }
}

pub struct ReadResourceWriterResource {
    state: RefCell<ReadResourceWriterState>,
}

struct ReadResourceWriterState {
    id: Id,
    out: OutputStream,
    uri: String,
    chunks: Vec<Vec<u8>>,
    options: Option<read_resource_writer::ResourceContentsOptions>,
}

impl read_resource_writer::GuestWriter for ReadResourceWriterResource {
    fn write(&self, contents: Vec<u8>) -> Result<(), StreamError> {
        let mut state = self.state.borrow_mut();
        state.chunks.push(contents);
        Ok(())
    }

    fn close(&self, options: Option<read_resource_writer::Options>) -> Result<(), StreamError> {
        let state = self.state.borrow();

        let mut result = JsonObjectBuilder::new();
        result.add_string("uri", &state.uri);

        // Combine all chunks efficiently
        let total_len: usize = state.chunks.iter().map(|c| c.len()).sum();
        let mut combined = Vec::with_capacity(total_len);
        for chunk in &state.chunks {
            combined.extend_from_slice(chunk);
        }

        // Check if it's text or binary
        if let Ok(text) = String::from_utf8(combined.clone()) {
            result.add_string("text", &text);
        } else {
            result.add_string("blob", &base64_encode(&combined));
        }

        // Add options from initial
        if let Some(opts) = &state.options {
            if let Some(mime) = &opts.mime_type {
                result.add_string("mimeType", mime);
            }
            if let Some(meta) = &opts.meta {
                if !meta.is_empty() {
                    result.add_field("_meta", &build_meta_object(meta));
                }
            }
        }

        // Override with close options if provided
        if let Some(opts) = options {
            if let Some(meta) = opts.meta {
                if !meta.is_empty() {
                    result.add_field("_meta", &build_meta_object(&meta));
                }
            }
        }

        let response = build_jsonrpc_response(&state.id, &result.build());
        write_sse_message(&state.out, &response)
    }
}

// ===== List Resource Templates Writer =====

impl list_resource_templates_writer::Guest for ListResourceTemplatesWriter {
    fn send(
        id: Id,
        out: OutputStream,
        templates: Vec<list_resource_templates_writer::ResourceTemplate>,
    ) -> Result<(), StreamError> {
        let templates_json = build_templates_array(&templates);

        let mut result = JsonObjectBuilder::new();
        result.add_field("resourceTemplates", &templates_json);

        let response = build_jsonrpc_response(&id, &result.build());
        write_sse_message(&out, &response)
    }

    type Writer = ListResourceTemplatesWriterResource;

    fn open(
        id: Id,
        out: OutputStream,
    ) -> Result<list_resource_templates_writer::Writer, StreamError> {
        Ok(list_resource_templates_writer::Writer::new(ListResourceTemplatesWriterResource {
            state: RefCell::new(TemplatesWriterState {
                id,
                out,
                templates: Vec::new(),
            }),
        }))
    }
}

pub struct ListResourceTemplatesWriterResource {
    state: RefCell<TemplatesWriterState>,
}

struct TemplatesWriterState {
    id: Id,
    out: OutputStream,
    templates: Vec<list_resource_templates_writer::ResourceTemplate>,
}

impl list_resource_templates_writer::GuestWriter for ListResourceTemplatesWriterResource {
    fn write(&self, template: list_resource_templates_writer::ResourceTemplate) -> Result<(), StreamError> {
        let mut state = self.state.borrow_mut();
        state.templates.push(template);
        Ok(())
    }

    fn close(&self, options: Option<list_resource_templates_writer::Options>) -> Result<(), StreamError> {
        let state = self.state.borrow();
        let templates_json = build_templates_array(&state.templates);

        let mut result = JsonObjectBuilder::new();
        result.add_field("resourceTemplates", &templates_json);

        if let Some(opts) = options {
            if let Some(meta) = opts.meta {
                if !meta.is_empty() {
                    result.add_field("_meta", &build_meta_object(&meta));
                }
            }
            if let Some(Some(cursor)) = opts.next_cursor.as_ref() {
                result.add_string("nextCursor", cursor);
            }
        }

        let response = build_jsonrpc_response(&state.id, &result.build());
        write_sse_message(&state.out, &response)
    }
}

// ===== Helper Functions =====

/// Build a JSON array of resources.
fn build_resources_array(resources: &[list_resources_writer::Resource]) -> String {
    if resources.is_empty() {
        return "[]".to_string();
    }

    let resource_objects: Vec<String> = resources.iter().map(|resource| {
        let mut obj = JsonObjectBuilder::new();
        obj.add_string("uri", &resource.uri);
        obj.add_string("name", &resource.name);

        if let Some(opts) = &resource.options {
            obj.add_optional_number("size", opts.size);
            obj.add_optional_string("title", opts.title.as_deref());
            obj.add_optional_string("description", opts.description.as_deref());
            obj.add_optional_string("mimeType", opts.mime_type.as_deref());

            if let Some(ann) = &opts.annotations {
                obj.add_field("annotations", &build_annotations_json(ann));
            }
            if let Some(meta) = &opts.meta {
                if !meta.is_empty() {
                    obj.add_field("_meta", &build_meta_object(meta));
                }
            }
        }

        obj.build()
    }).collect();

    format!("[{}]", resource_objects.join(","))
}

/// Build a JSON array of resource templates.
fn build_templates_array(templates: &[list_resource_templates_writer::ResourceTemplate]) -> String {
    if templates.is_empty() {
        return "[]".to_string();
    }

    let template_objects: Vec<String> = templates.iter().map(|template| {
        let mut obj = JsonObjectBuilder::new();
        obj.add_string("uriTemplate", &template.uri_template);
        obj.add_string("name", &template.name);

        if let Some(opts) = &template.options {
            obj.add_optional_string("description", opts.description.as_deref());
            obj.add_optional_string("title", opts.title.as_deref());
            obj.add_optional_string("mimeType", opts.mime_type.as_deref());

            if let Some(ann) = &opts.annotations {
                obj.add_field("annotations", &build_annotations_json(ann));
            }
            if let Some(meta) = &opts.meta {
                if !meta.is_empty() {
                    obj.add_field("_meta", &build_meta_object(meta));
                }
            }
        }

        obj.build()
    }).collect();

    format!("[{}]", template_objects.join(","))
}