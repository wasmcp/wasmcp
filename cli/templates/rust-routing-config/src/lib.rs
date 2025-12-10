//! {{project_name}} Routing Configuration Provider
//!
//! Provides routing configuration for filter-middleware.
//! The routing.toml file is embedded at compile time and served as a resource.

mod bindings {
    wit_bindgen::generate!({
        world: "{{project_name}}",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp_v20250618::resources::Guest;
use bindings::wasmcp::mcp_v20250618::mcp::*;
use bindings::wasmcp::mcp_v20250618::server_handler::MessageContext;

struct RoutingConfig;

impl Guest for RoutingConfig {
    fn list_resources(
        _ctx: MessageContext,
        _req: ListResourcesRequest,
    ) -> Result<ListResourcesResult, ErrorCode> {
        Ok(ListResourcesResult {
            resources: vec![McpResource {
                uri: "config://routing-{{project_name}}".to_string(),
                name: "{{project_name}} Routing Configuration".to_string(),
                options: Some(ResourceOptions {
                    size: None,
                    title: Some("Routing Configuration".to_string()),
                    description: Some(
                        "Path and tag-based routing rules for filter-middleware".to_string(),
                    ),
                    mime_type: Some("application/toml".to_string()),
                    annotations: None,
                    meta: None,
                }),
            }],
            next_cursor: None,
            meta: None,
        })
    }

    fn read_resource(
        _ctx: MessageContext,
        request: ReadResourceRequest,
    ) -> Result<Option<ReadResourceResult>, ErrorCode> {
        if request.uri == "config://routing-{{project_name}}" {
            // Embed routing.toml at compile time
            let config_toml = include_str!("../routing.toml");

            Ok(Some(ReadResourceResult {
                meta: None,
                contents: vec![ResourceContents::Text(TextResourceContents {
                    uri: request.uri,
                    text: TextData::Text(config_toml.to_string()),
                    options: Some(EmbeddedResourceOptions {
                        mime_type: Some("application/toml".to_string()),
                        meta: None,
                    }),
                })],
            }))
        } else {
            Ok(None)
        }
    }

    fn list_resource_templates(
        _ctx: MessageContext,
        _req: ListResourceTemplatesRequest,
    ) -> Result<ListResourceTemplatesResult, ErrorCode> {
        // No templates
        Ok(ListResourceTemplatesResult {
            meta: None,
            next_cursor: None,
            resource_templates: vec![],
        })
    }
}

bindings::export!(RoutingConfig with_types_in bindings);
