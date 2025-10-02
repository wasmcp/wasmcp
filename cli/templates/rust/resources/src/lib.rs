//! {{ handler_type_capitalized }} handler for MCP.

#[allow(warnings)]
mod bindings;

use bindings::exports::wasmcp::mcp::incoming_handler::{Guest, OutputStream, Request};
use bindings::wasmcp::mcp::incoming_handler as next_handler;
use bindings::wasmcp::mcp::resources_list_result;
use bindings::wasmcp::mcp::resources_read_result;
use bindings::wasmcp::mcp::error_result;
use bindings::wasmcp::mcp::request::{Params, ServerCapabilities};

pub struct Component;

impl Component {
    fn handle_resources_list(request: &Request, output: OutputStream) {
        let id = request.id();

        let resources = vec![resources_list_result::Resource {
            uri: "file:///example.txt".to_string(),
            name: "example.txt".to_string(),
            options: Some(resources_list_result::ResourceOptions {
                size: None,
                title: None,
                description: Some("An example text resource".to_string()),
                mime_type: Some("text/plain".to_string()),
                annotations: None,
                meta: None,
            }),
        }];

        if let Err(e) = resources_list_result::write(&id, output, &resources, None) {
            eprintln!("Failed to write resources list: {:?}", e);
        }
    }

    fn handle_resources_read(request: &Request, uri: String, output: OutputStream) {
        let id = request.id();

        let content = if uri == "file:///example.txt" {
            Self::read_example()
        } else {
            format!("Unknown resource: {}", uri)
        };

        let contents = resources_read_result::Contents {
            uri,
            data: content.into_bytes(),
            options: Some(resources_read_result::ContentsOptions {
                mime_type: Some("text/plain".to_string()),
                meta: None,
            }),
        };

        if let Err(e) = resources_read_result::write(&id, output, &contents, None) {
            eprintln!("Failed to write resource: {:?}", e);
        }
    }

    fn read_example() -> String {
        "This is the content of example.txt".to_string()
    }
}

impl Guest for Component {
    fn handle(request: Request, output: OutputStream) {
        if !request.needs(ServerCapabilities::RESOURCES) {
            next_handler::handle(request, output);
            return;
        }

        match request.params() {
            Ok(Params::ResourcesList(_)) => Self::handle_resources_list(&request, output),
            Ok(Params::ResourcesRead(uri)) => Self::handle_resources_read(&request, uri, output),
            Ok(_) => unreachable!(),
            Err(error) => {
                let _ = error_result::write(&request.id(), output, &error);
            }
        }
    }
}

bindings::export!(Component with_types_in bindings);
