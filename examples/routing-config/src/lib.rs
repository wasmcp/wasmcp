mod bindings {
    wit_bindgen::generate!({
        world: "routing-config",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp_v20250618::resources::Guest;
use bindings::wasmcp::mcp_v20250618::mcp::*;
use bindings::wasmcp::mcp_v20250618::server_handler::MessageContext;

const ROUTING_URI: &str = "config://routing-config";
const ROUTING_OVERRIDE_URI: &str = "config://routing-team-override";

struct RoutingConfig;

impl Guest for RoutingConfig {
    fn list_resources(
        _ctx: MessageContext,
        _req: ListResourcesRequest,
    ) -> Result<ListResourcesResult, ErrorCode> {
        Ok(ListResourcesResult {
            resources: vec![
                McpResource {
                    uri: ROUTING_URI.to_string(),
                    name: "Routing Configuration".to_string(),
                    options: Some(ResourceOptions {
                        size: None,
                        title: Some("Routing Configuration".to_string()),
                        description: Some("Path and tag-based routing rules".to_string()),
                        mime_type: Some("application/toml".to_string()),
                        annotations: None,
                        meta: None,
                    }),
                },
                McpResource {
                    uri: ROUTING_OVERRIDE_URI.to_string(),
                    name: "Routing Configuration Override".to_string(),
                    options: Some(ResourceOptions {
                        size: None,
                        title: Some("Routing Configuration Override".to_string()),
                        description: Some(
                            "Override routing rules that demonstrate multi-config aggregation"
                                .to_string(),
                        ),
                        mime_type: Some("application/toml".to_string()),
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
        _ctx: MessageContext,
        request: ReadResourceRequest,
    ) -> Result<Option<ReadResourceResult>, ErrorCode> {
        match request.uri.as_str() {
            ROUTING_URI => {
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
            }
            ROUTING_OVERRIDE_URI => {
                // Embed routing-override.toml at compile time
                let override_toml = include_str!("../routing-override.toml");

                Ok(Some(ReadResourceResult {
                    meta: None,
                    contents: vec![ResourceContents::Text(TextResourceContents {
                        uri: request.uri,
                        text: TextData::Text(override_toml.to_string()),
                        options: Some(EmbeddedResourceOptions {
                            mime_type: Some("application/toml".to_string()),
                            meta: None,
                        }),
                    })],
                }))
            }
            _ => Ok(None),
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
