//! {{project_name}} Resources Capability Provider
//!
//! A resources capability that provides simple text resources.

mod bindings {
    wit_bindgen::generate!({
        world: "{{project_name}}",
        generate_all,
    });
}

use bindings::exports::wasmcp::protocol::resources::Guest;
use bindings::wasmcp::protocol::mcp::*;
use bindings::wasi::io::streams::OutputStream;

struct TextResources;

impl Guest for TextResources {
    fn list_resources(
        _ctx: bindings::wasmcp::protocol::server_messages::Context,
        _request: ListResourcesRequest,
        _client_stream: Option<&OutputStream>,
    ) -> Result<ListResourcesResult, ErrorCode> {
        Ok(ListResourcesResult {
            resources: vec![
                McpResource {
                    uri: "text://greeting".to_string(),
                    name: "Greeting".to_string(),
                    options: Some(ResourceOptions {
                        size: None,
                        title: Some("Greeting".to_string()),
                        description: Some("A friendly greeting message".to_string()),
                        mime_type: Some("text/plain".to_string()),
                        annotations: None,
                        meta: None,
                    }),
                },
                McpResource {
                    uri: "text://info".to_string(),
                    name: "Info".to_string(),
                    options: Some(ResourceOptions {
                        size: None,
                        title: Some("Info".to_string()),
                        description: Some("Information about this resource provider".to_string()),
                        mime_type: Some("text/plain".to_string()),
                        annotations: None,
                        meta: None,
                    }),
                },
            ],
            next_cursor: None,
            meta: None,
        })
    }

    fn read_resource(
        _ctx: bindings::wasmcp::protocol::server_messages::Context,
        request: ReadResourceRequest,
        _client_stream: Option<&OutputStream>,
    ) -> Option<ReadResourceResult> {
        match request.uri.as_str() {
            "text://greeting" => Some(success_result("Hello from wasmcp resources!")),
            "text://info" => Some(success_result(
                "This is a simple resources capability component. \
                 It provides static text content via custom URIs.",
            )),
            _ => None, // We don't handle this URI
        }
    }

    fn list_resource_templates(
        _ctx: bindings::wasmcp::protocol::server_messages::Context,
        _request: ListResourceTemplatesRequest,
        _client_stream: Option<&OutputStream>,
    ) -> Result<ListResourceTemplatesResult, ErrorCode> {
        // No templates for static resources
        Ok(ListResourceTemplatesResult {
            resource_templates: vec![],
            next_cursor: None,
            meta: None,
        })
    }
}

fn success_result(text: &str) -> ReadResourceResult {
    ReadResourceResult {
        contents: vec![ResourceContents::Text(TextResourceContents {
            uri: String::new(), // URI is provided in request
            text: TextData::Text(text.to_string()),
            options: Some(EmbeddedResourceOptions {
                mime_type: Some("text/plain".to_string()),
                meta: None,
            }),
        })],
        meta: None,
    }
}

bindings::export!(TextResources with_types_in bindings);
