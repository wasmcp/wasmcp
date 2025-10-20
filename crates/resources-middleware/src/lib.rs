//! Resources Middleware Component
//!
//! A reusable middleware that bridges the MCP protocol (server-handler)
//! with the clean resources-capability interface. This component:
//! - Detects resources/list, resources/read, and resources/templates/list requests
//! - Calls the imported resources-capability functions
//! - Merges results with downstream handlers
//! - Delegates all other requests downstream

#![allow(warnings)]

mod bindings {
    wit_bindgen::generate!({
        world: "resources-middleware",
        generate_all,
    });
}

use bindings::exports::wasmcp::server::handler::Guest;
use bindings::wasi::io::streams::OutputStream;
use bindings::wasmcp::protocol::mcp::*;
use bindings::wasmcp::protocol::resources as capability;
use bindings::wasmcp::protocol::server_messages::Context;
use bindings::wasmcp::server::handler as downstream;

struct ResourcesMiddleware;

impl Guest for ResourcesMiddleware {
    fn handle_request(
        ctx: Context,
        request: (ClientRequest, RequestId),
        client_stream: Option<&OutputStream>,
    ) -> Result<ServerResponse, ErrorCode> {
        let (req, id) = request;
        match req {
            ClientRequest::ResourcesList(list_req) => {
                handle_resources_list(list_req, id, &ctx, client_stream)
            }
            ClientRequest::ResourcesRead(read_req) => {
                handle_resources_read(read_req, id, &ctx, client_stream)
            }
            ClientRequest::ResourcesTemplatesList(templates_req) => {
                handle_templates_list(templates_req, id, &ctx, client_stream)
            }
            _ => {
                // Delegate all other requests to downstream handler
                downstream::handle_request(&ctx, (&req, &id), client_stream)
            }
        }
    }

    fn handle_notification(ctx: Context, notification: ClientNotification) {
        // Forward to downstream handler
        downstream::handle_notification(&ctx, &notification);
    }

    fn handle_response(ctx: Context, response: Result<(ClientResponse, RequestId), ErrorCode>) {
        // Forward to downstream handler
        downstream::handle_response(&ctx, response);
    }
}

fn handle_resources_list(
    req: ListResourcesRequest,
    id: RequestId,
    ctx: &Context,
    client_stream: Option<&OutputStream>,
) -> Result<ServerResponse, ErrorCode> {
    use bindings::wasmcp::protocol::mcp::ListResourcesResult;

    // Try to get resources from our capability
    let our_result = match capability::list_resources(ctx, &req, client_stream) {
        Ok(result) => Some(result),
        Err(ErrorCode::MethodNotFound(_)) => {
            // Capability doesn't implement resources interface - skip it
            None
        }
        Err(e) => {
            // Real error (InvalidParams, InternalError, etc.) - return it
            // Don't hide capability errors by silently falling back to downstream
            return Err(e);
        }
    };

    // Try to get downstream resources
    let downstream_req = ClientRequest::ResourcesList(req.clone());
    match downstream::handle_request(ctx, (&downstream_req, &id), client_stream) {
        Ok(ServerResponse::ResourcesList(downstream_result)) => {
            // Merge our resources with downstream resources
            match our_result {
                Some(our) => {
                    let mut all_resources = our.resources;
                    all_resources.extend(downstream_result.resources);

                    Ok(ServerResponse::ResourcesList(ListResourcesResult {
                        resources: all_resources,
                        next_cursor: downstream_result.next_cursor.or(our.next_cursor),
                        meta: our.meta.or(downstream_result.meta),
                    }))
                }
                None => {
                    // Only downstream has resources
                    Ok(ServerResponse::ResourcesList(downstream_result))
                }
            }
        }
        Err(ErrorCode::MethodNotFound(_)) => {
            // Downstream doesn't support resources
            match our_result {
                Some(our) => Ok(ServerResponse::ResourcesList(our)),
                None => {
                    // Neither capability nor downstream implements resources - return MethodNotFound
                    Err(ErrorCode::MethodNotFound(Error {
                        id: Some(id),
                        code: -32601,
                        message: "Method not found: resources/list".to_string(),
                        data: None,
                    }))
                }
            }
        }
        Err(e) => {
            // Downstream returned a real error
            match our_result {
                Some(our) => {
                    // We have capability resources, return them despite downstream error
                    Ok(ServerResponse::ResourcesList(our))
                }
                None => {
                    // No capability resources, propagate downstream error
                    Err(e)
                }
            }
        }
        Ok(_) => {
            // Unexpected response type from downstream
            match our_result {
                Some(our) => Ok(ServerResponse::ResourcesList(our)),
                None => {
                    // No resources available
                    Err(ErrorCode::InternalError(Error {
                        id: Some(id),
                        code: -32603,
                        message: "Unexpected response type from downstream handler".to_string(),
                        data: None,
                    }))
                }
            }
        }
    }
}

fn handle_resources_read(
    req: ReadResourceRequest,
    id: RequestId,
    ctx: &Context,
    client_stream: Option<&OutputStream>,
) -> Result<ServerResponse, ErrorCode> {
    // Try reading from our capability first
    match capability::read_resource(ctx, &req, client_stream) {
        Some(result) => {
            // Capability handled it - return the result
            Ok(ServerResponse::ResourcesRead(result))
        }
        None => {
            // Capability doesn't handle this URI - try downstream
            let downstream_req = ClientRequest::ResourcesRead(req.clone());
            match downstream::handle_request(ctx, (&downstream_req, &id), client_stream) {
                Ok(response) => Ok(response),
                Err(ErrorCode::MethodNotFound(_)) => {
                    // Downstream also doesn't handle it - return InvalidParams
                    // The method exists, but the URI parameter is invalid/unknown
                    Err(ErrorCode::InvalidParams(Error {
                        id: Some(id),
                        code: -32602,
                        message: format!("Unknown resource URI: {}", req.uri),
                        data: None,
                    }))
                }
                Err(e) => Err(e),
            }
        }
    }
}

fn handle_templates_list(
    req: ListResourceTemplatesRequest,
    id: RequestId,
    ctx: &Context,
    client_stream: Option<&OutputStream>,
) -> Result<ServerResponse, ErrorCode> {
    use bindings::wasmcp::protocol::mcp::ListResourceTemplatesResult;

    // Try to get templates from our capability
    let our_result = match capability::list_resource_templates(ctx, &req, client_stream) {
        Ok(result) => Some(result),
        Err(ErrorCode::MethodNotFound(_)) => {
            // Capability doesn't implement templates - skip it
            None
        }
        Err(e) => {
            // Real error - return it
            return Err(e);
        }
    };

    // Try to get downstream templates
    let downstream_req = ClientRequest::ResourcesTemplatesList(req.clone());
    match downstream::handle_request(ctx, (&downstream_req, &id), client_stream) {
        Ok(ServerResponse::ResourcesTemplatesList(downstream_result)) => {
            // Merge our templates with downstream templates
            match our_result {
                Some(our) => {
                    let mut all_templates = our.resource_templates;
                    all_templates.extend(downstream_result.resource_templates);

                    Ok(ServerResponse::ResourcesTemplatesList(
                        ListResourceTemplatesResult {
                            resource_templates: all_templates,
                            next_cursor: downstream_result.next_cursor.or(our.next_cursor),
                            meta: our.meta.or(downstream_result.meta),
                        },
                    ))
                }
                None => {
                    // Only downstream has templates
                    Ok(ServerResponse::ResourcesTemplatesList(downstream_result))
                }
            }
        }
        Err(ErrorCode::MethodNotFound(_)) => {
            // Downstream doesn't support templates
            match our_result {
                Some(our) => Ok(ServerResponse::ResourcesTemplatesList(our)),
                None => {
                    // Neither capability nor downstream implements templates - return MethodNotFound
                    Err(ErrorCode::MethodNotFound(Error {
                        id: Some(id),
                        code: -32601,
                        message: "Method not found: resources/templates/list".to_string(),
                        data: None,
                    }))
                }
            }
        }
        Err(e) => {
            // Downstream returned a real error
            match our_result {
                Some(our) => {
                    // We have capability templates, return them despite downstream error
                    Ok(ServerResponse::ResourcesTemplatesList(our))
                }
                None => {
                    // No capability templates, propagate downstream error
                    Err(e)
                }
            }
        }
        Ok(_) => {
            // Unexpected response type from downstream
            match our_result {
                Some(our) => Ok(ServerResponse::ResourcesTemplatesList(our)),
                None => {
                    // No templates available
                    Err(ErrorCode::InternalError(Error {
                        id: Some(id),
                        code: -32603,
                        message: "Unexpected response type from downstream handler".to_string(),
                        data: None,
                    }))
                }
            }
        }
    }
}

bindings::export!(ResourcesMiddleware with_types_in bindings);
