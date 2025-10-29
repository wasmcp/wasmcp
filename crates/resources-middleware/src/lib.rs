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

use bindings::exports::wasmcp::mcp_v20250618::server_handler::{
    ErrorCtx as ExportErrorCtx, Guest, NotificationCtx as ExportNotificationCtx,
    RequestCtx as ExportRequestCtx, ResultCtx as ExportResultCtx,
};
use bindings::wasi::io::streams::OutputStream;
use bindings::wasmcp::mcp_v20250618::mcp::*;
use bindings::wasmcp::mcp_v20250618::resources as capability;
use bindings::wasmcp::mcp_v20250618::server_handler as downstream;

struct ResourcesMiddleware;

impl Guest for ResourcesMiddleware {
    fn handle_request(
        ctx: ExportRequestCtx,
        request: ClientRequest,
    ) -> Result<ServerResult, ErrorCode> {
        match &request {
            ClientRequest::ResourcesList(list_req) => handle_resources_list(list_req.clone(), ctx),
            ClientRequest::ResourcesRead(read_req) => handle_resources_read(read_req.clone(), ctx),
            ClientRequest::ResourcesTemplatesList(templates_req) => {
                handle_templates_list(templates_req.clone(), ctx)
            }
            _ => {
                // Delegate all other requests to downstream handler
                downstream::handle_request(
                    &downstream::RequestCtx {
                        request_id: ctx.request_id.clone(),
                        jwt: ctx.jwt.clone(),
                        session_id: ctx.session_id.clone(),
                        message_stream: ctx.message_stream,
                        protocol_version: ctx.protocol_version.clone(),
                    },
                    &request,
                )
            }
        }
    }

    fn handle_notification(ctx: ExportNotificationCtx, notification: ClientNotification) {
        // Forward to downstream handler
        downstream::handle_notification(
            &downstream::NotificationCtx {
                jwt: ctx.jwt.clone(),
                session_id: ctx.session_id.clone(),
                protocol_version: ctx.protocol_version.clone(),
            },
            &notification,
        );
    }

    fn handle_result(ctx: ExportResultCtx, result: ClientResult) {
        // Forward to downstream handler
        downstream::handle_result(
            &downstream::ResultCtx {
                request_id: ctx.request_id.clone(),
                jwt: ctx.jwt.clone(),
                session_id: ctx.session_id.clone(),
                protocol_version: ctx.protocol_version.clone(),
            },
            result,
        );
    }

    fn handle_error(ctx: ExportErrorCtx, error: ErrorCode) {
        // Forward to downstream handler
        downstream::handle_error(
            &downstream::ErrorCtx {
                request_id: ctx.request_id.clone(),
                jwt: ctx.jwt.clone(),
                session_id: ctx.session_id.clone(),
                protocol_version: ctx.protocol_version.clone(),
            },
            &error,
        );
    }
}

fn handle_resources_list(
    req: ListResourcesRequest,
    ctx: ExportRequestCtx,
) -> Result<ServerResult, ErrorCode> {
    // Try to get resources from our capability
    let our_result = match capability::list_resources(
        &capability::RequestCtx {
            request_id: ctx.request_id.clone(),
            jwt: ctx.jwt.clone(),
            session_id: ctx.session_id.clone(),
            message_stream: ctx.message_stream,
            protocol_version: ctx.protocol_version.clone(),
        },
        &req,
    ) {
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
    match downstream::handle_request(
        &downstream::RequestCtx {
            request_id: ctx.request_id.clone(),
            jwt: ctx.jwt.clone(),
            session_id: ctx.session_id.clone(),
            message_stream: ctx.message_stream,
            protocol_version: ctx.protocol_version.clone(),
        },
        &downstream_req,
    ) {
        Ok(ServerResult::ResourcesList(downstream_result)) => {
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
        Err(ErrorCode::MethodNotFound(_)) => {
            // Downstream doesn't support resources
            match our_result {
                Some(our) => Ok(ServerResult::ResourcesList(our)),
                None => {
                    // Neither capability nor downstream implements resources - return MethodNotFound
                    Err(ErrorCode::MethodNotFound(Error {
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
                    Ok(ServerResult::ResourcesList(our))
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
    }
}

fn handle_resources_read(
    req: ReadResourceRequest,
    ctx: ExportRequestCtx,
) -> Result<ServerResult, ErrorCode> {
    // Try reading from our capability first
    match capability::read_resource(
        &capability::RequestCtx {
            request_id: ctx.request_id.clone(),
            jwt: ctx.jwt.clone(),
            session_id: ctx.session_id.clone(),
            message_stream: ctx.message_stream,
            protocol_version: ctx.protocol_version.clone(),
        },
        &req,
    ) {
        Ok(Some(result)) => {
            // Capability handled it - return the result
            Ok(ServerResult::ResourcesRead(result))
        }
        Ok(None) => {
            // Capability doesn't handle this URI - try downstream
            let downstream_req = ClientRequest::ResourcesRead(req.clone());
            match downstream::handle_request(
                &downstream::RequestCtx {
                    request_id: ctx.request_id.clone(),
                    jwt: ctx.jwt.clone(),
                    session_id: ctx.session_id.clone(),
                    message_stream: ctx.message_stream,
                    protocol_version: ctx.protocol_version.clone(),
                },
                &downstream_req,
            ) {
                Ok(response) => Ok(response),
                Err(ErrorCode::MethodNotFound(_)) => {
                    // Downstream also doesn't handle it - return InvalidParams
                    // The method exists, but the URI parameter is invalid/unknown
                    Err(ErrorCode::InvalidParams(Error {
                        code: -32602,
                        message: format!("Unknown resource URI: {}", req.uri),
                        data: None,
                    }))
                }
                Err(e) => Err(e),
            }
        }
        Err(e) => {
            // Capability returned an error - propagate it
            Err(e)
        }
    }
}

fn handle_templates_list(
    req: ListResourceTemplatesRequest,
    ctx: ExportRequestCtx,
) -> Result<ServerResult, ErrorCode> {
    // Try to get templates from our capability
    let our_result = match capability::list_resource_templates(
        &capability::RequestCtx {
            request_id: ctx.request_id.clone(),
            jwt: ctx.jwt.clone(),
            session_id: ctx.session_id.clone(),
            message_stream: ctx.message_stream,
            protocol_version: ctx.protocol_version.clone(),
        },
        &req,
    ) {
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
    match downstream::handle_request(
        &downstream::RequestCtx {
            request_id: ctx.request_id.clone(),
            jwt: ctx.jwt.clone(),
            session_id: ctx.session_id.clone(),
            message_stream: ctx.message_stream,
            protocol_version: ctx.protocol_version.clone(),
        },
        &downstream_req,
    ) {
        Ok(ServerResult::ResourcesTemplatesList(downstream_result)) => {
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
        Err(ErrorCode::MethodNotFound(_)) => {
            // Downstream doesn't support templates
            match our_result {
                Some(our) => Ok(ServerResult::ResourcesTemplatesList(our)),
                None => {
                    // Neither capability nor downstream implements templates - return MethodNotFound
                    Err(ErrorCode::MethodNotFound(Error {
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
                    Ok(ServerResult::ResourcesTemplatesList(our))
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
    }
}

bindings::export!(ResourcesMiddleware with_types_in bindings);
