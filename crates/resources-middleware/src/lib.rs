//! Resources Middleware Component
//!
//! A reusable middleware that bridges the MCP protocol (server-handler)
//! with the resources interface. This component:
//! - Detects resources/list, resources/read, and resources/templates/list requests
//! - Calls the imported resources interface functions
//! - Merges results with downstream handlers
//! - Delegates all other requests downstream

#![allow(warnings)]

mod bindings {
    wit_bindgen::generate!({
        world: "resources-middleware",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp_v20250618::server_handler::{Guest, MessageContext};
use bindings::wasmcp::mcp_v20250618::mcp::*;
use bindings::wasmcp::mcp_v20250618::resources;
use bindings::wasmcp::mcp_v20250618::server_handler as downstream;

struct ResourcesMiddleware;

// Convert exported MessageContext to imported MessageContext
fn to_downstream_ctx<'a>(ctx: &'a MessageContext<'a>) -> downstream::MessageContext<'a> {
    downstream::MessageContext {
        client_stream: ctx.client_stream,
        protocol_version: ctx.protocol_version.clone(),
        session: ctx.session.as_ref().map(|s| downstream::Session {
            session_id: s.session_id.clone(),
            store_id: s.store_id.clone(),
        }),
        identity: ctx.identity.as_ref().map(|i| downstream::Identity {
            jwt: i.jwt.clone(),
            claims: i.claims.clone(),
        }),
        frame: ctx.frame.clone(),
    }
}

impl Guest for ResourcesMiddleware {
    fn handle(
        ctx: MessageContext,
        message: ClientMessage,
    ) -> Option<Result<ServerResult, ErrorCode>> {
        match message {
            ClientMessage::Request((request_id, request)) => {
                // Handle requests - match on request type
                let result = match &request {
                    ClientRequest::ResourcesList(list_req) => {
                        handle_resources_list(list_req.clone(), &ctx)
                    }
                    ClientRequest::ResourcesRead(read_req) => {
                        handle_resources_read(read_req.clone(), &ctx)
                    }
                    ClientRequest::ResourcesTemplatesList(templates_req) => {
                        handle_templates_list(templates_req.clone(), &ctx)
                    }
                    _ => {
                        // Delegate all other requests to downstream handler
                        let downstream_msg = ClientMessage::Request((request_id.clone(), request));
                        return downstream::handle(&to_downstream_ctx(&ctx), downstream_msg);
                    }
                };
                Some(result)
            }
            _ => {
                // Forward notifications, results, errors to downstream
                downstream::handle(&to_downstream_ctx(&ctx), message)
            }
        }
    }
}

fn handle_resources_list(
    req: ListResourcesRequest,
    ctx: &MessageContext,
) -> Result<ServerResult, ErrorCode> {
    // Try to get resources from imported resources interface
    let our_result = match resources::list_resources(&to_downstream_ctx(ctx), &req) {
        Ok(result) => Some(result),
        Err(ErrorCode::MethodNotFound(_)) => {
            // Component doesn't implement resources interface - skip it
            None
        }
        Err(e) => {
            // Real error (InvalidParams, InternalError, etc.) - return it
            // Don't hide errors by silently falling back to downstream
            return Err(e);
        }
    };

    // Try to get downstream resources
    let downstream_req = ClientRequest::ResourcesList(req.clone());
    let downstream_msg = ClientMessage::Request((RequestId::Number(0), downstream_req));
    match downstream::handle(&to_downstream_ctx(ctx), downstream_msg) {
        Some(Ok(ServerResult::ResourcesList(downstream_result))) => {
            // Merge our resources with downstream resources
            match our_result {
                Some(our) => {
                    let mut all_resources = our.resources;
                    all_resources.extend(downstream_result.resources);

                    Ok(ServerResult::ResourcesList(ListResourcesResult {
                        resources: all_resources,
                        next_cursor: downstream_result.next_cursor.or(our.next_cursor),
                        meta: our.meta.or(downstream_result.meta),
                    }))
                }
                None => {
                    // Only downstream has resources
                    Ok(ServerResult::ResourcesList(downstream_result))
                }
            }
        }
        Some(Err(ErrorCode::MethodNotFound(_))) => {
            // Downstream doesn't support resources
            match our_result {
                Some(our) => Ok(ServerResult::ResourcesList(our)),
                None => {
                    // Neither component nor downstream implements resources - return MethodNotFound
                    Err(ErrorCode::MethodNotFound(Error {
                        code: -32601,
                        message: "Method not found: resources/list".to_string(),
                        data: None,
                    }))
                }
            }
        }
        Some(Err(e)) => {
            // Downstream returned a real error
            match our_result {
                Some(our) => {
                    // We have resources from imported interface, return them despite downstream error
                    Ok(ServerResult::ResourcesList(our))
                }
                None => {
                    // No resources from imported interface, propagate downstream error
                    Err(e)
                }
            }
        }
        Some(Ok(_)) => {
            // Unexpected response type from downstream
            match our_result {
                Some(our) => Ok(ServerResult::ResourcesList(our)),
                None => {
                    // No resources available
                    Err(ErrorCode::InternalError(Error {
                        code: -32603,
                        message: "Unexpected response type from downstream handler".to_string(),
                        data: None,
                    }))
                }
            }
        }
        None => {
            // Downstream returned None (no response for non-request)
            match our_result {
                Some(our) => Ok(ServerResult::ResourcesList(our)),
                None => Err(ErrorCode::MethodNotFound(Error {
                    code: -32601,
                    message: "Method not found: resources/list".to_string(),
                    data: None,
                })),
            }
        }
    }
}

fn handle_resources_read(
    req: ReadResourceRequest,
    ctx: &MessageContext,
) -> Result<ServerResult, ErrorCode> {
    // Try reading from imported resources interface first
    match resources::read_resource(&to_downstream_ctx(ctx), &req) {
        Ok(Some(result)) => {
            // Imported interface handled it - return the result
            Ok(ServerResult::ResourcesRead(result))
        }
        Ok(None) => {
            // Imported interface doesn't handle this URI - try downstream
            let downstream_req = ClientRequest::ResourcesRead(req.clone());
            let downstream_msg = ClientMessage::Request((RequestId::Number(0), downstream_req));
            match downstream::handle(&to_downstream_ctx(ctx), downstream_msg) {
                Some(Ok(response)) => Ok(response),
                Some(Err(ErrorCode::MethodNotFound(_))) | None => {
                    // Downstream also doesn't handle it - return InvalidParams
                    // The method exists, but the URI parameter is invalid/unknown
                    Err(ErrorCode::InvalidParams(Error {
                        code: -32602,
                        message: format!("Unknown resource URI: {}", req.uri),
                        data: None,
                    }))
                }
                Some(Err(e)) => Err(e),
            }
        }
        Err(e) => {
            // Imported interface returned an error - propagate it
            Err(e)
        }
    }
}

fn handle_templates_list(
    req: ListResourceTemplatesRequest,
    ctx: &MessageContext,
) -> Result<ServerResult, ErrorCode> {
    // Try to get templates from imported resources interface
    let our_result = match resources::list_resource_templates(&to_downstream_ctx(ctx), &req) {
        Ok(result) => Some(result),
        Err(ErrorCode::MethodNotFound(_)) => {
            // Component doesn't implement templates - skip it
            None
        }
        Err(e) => {
            // Real error - return it
            return Err(e);
        }
    };

    // Try to get downstream templates
    let downstream_req = ClientRequest::ResourcesTemplatesList(req.clone());
    let downstream_msg = ClientMessage::Request((RequestId::Number(0), downstream_req));
    match downstream::handle(&to_downstream_ctx(ctx), downstream_msg) {
        Some(Ok(ServerResult::ResourcesTemplatesList(downstream_result))) => {
            // Merge our templates with downstream templates
            match our_result {
                Some(our) => {
                    let mut all_templates = our.resource_templates;
                    all_templates.extend(downstream_result.resource_templates);

                    Ok(ServerResult::ResourcesTemplatesList(
                        ListResourceTemplatesResult {
                            resource_templates: all_templates,
                            next_cursor: downstream_result.next_cursor.or(our.next_cursor),
                            meta: our.meta.or(downstream_result.meta),
                        },
                    ))
                }
                None => {
                    // Only downstream has templates
                    Ok(ServerResult::ResourcesTemplatesList(downstream_result))
                }
            }
        }
        Some(Err(ErrorCode::MethodNotFound(_))) => {
            // Downstream doesn't support templates
            match our_result {
                Some(our) => Ok(ServerResult::ResourcesTemplatesList(our)),
                None => {
                    // Neither component nor downstream implements templates - return MethodNotFound
                    Err(ErrorCode::MethodNotFound(Error {
                        code: -32601,
                        message: "Method not found: resources/templates/list".to_string(),
                        data: None,
                    }))
                }
            }
        }
        Some(Err(e)) => {
            // Downstream returned a real error
            match our_result {
                Some(our) => {
                    // We have templates from imported interface, return them despite downstream error
                    Ok(ServerResult::ResourcesTemplatesList(our))
                }
                None => {
                    // No templates from imported interface, propagate downstream error
                    Err(e)
                }
            }
        }
        Some(Ok(_)) => {
            // Unexpected response type from downstream
            match our_result {
                Some(our) => Ok(ServerResult::ResourcesTemplatesList(our)),
                None => {
                    // No templates available
                    Err(ErrorCode::InternalError(Error {
                        code: -32603,
                        message: "Unexpected response type from downstream handler".to_string(),
                        data: None,
                    }))
                }
            }
        }
        None => {
            // Downstream returned None (no response for non-request)
            match our_result {
                Some(our) => Ok(ServerResult::ResourcesTemplatesList(our)),
                None => Err(ErrorCode::MethodNotFound(Error {
                    code: -32601,
                    message: "Method not found: resources/templates/list".to_string(),
                    data: None,
                })),
            }
        }
    }
}

bindings::export!(ResourcesMiddleware with_types_in bindings);
