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

use bindings::exports::wasmcp::mcp_v20250618::server_handler::{
    ErrorCtx as ExportErrorCtx, Guest, Identity as ExportIdentity,
    NotificationCtx as ExportNotificationCtx, RequestCtx as ExportRequestCtx,
    ResultCtx as ExportResultCtx, Session as ExportSession,
};
use bindings::wasi::io::streams::OutputStream;
use bindings::wasmcp::mcp_v20250618::mcp::*;
use bindings::wasmcp::mcp_v20250618::resources;
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
                        id: ctx.id.clone(),
                        protocol_version: ctx.protocol_version.clone(),
                        messages: ctx.messages,
                        session: ctx.session.as_ref().map(|s| downstream::Session {
                            session_id: s.session_id.clone(),
                            store_id: s.store_id.clone(),
                        }),
                        user: ctx.user.as_ref().map(|u| downstream::Identity {
                            jwt: u.jwt.clone(),
                            claims: u.claims.clone(),
                        }),
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
                protocol_version: ctx.protocol_version.clone(),
                session: ctx.session.as_ref().map(|s| downstream::Session {
                    session_id: s.session_id.clone(),
                    store_id: s.store_id.clone(),
                }),
                user: ctx.user.as_ref().map(|u| downstream::Identity {
                    jwt: u.jwt.clone(),
                    claims: u.claims.clone(),
                }),
            },
            &notification,
        );
    }

    fn handle_result(ctx: ExportResultCtx, result: ClientResult) {
        // Forward to downstream handler
        downstream::handle_result(
            &downstream::ResultCtx {
                id: ctx.id.clone(),
                protocol_version: ctx.protocol_version.clone(),
                session: ctx.session.as_ref().map(|s| downstream::Session {
                    session_id: s.session_id.clone(),
                    store_id: s.store_id.clone(),
                }),
                user: ctx.user.as_ref().map(|u| downstream::Identity {
                    jwt: u.jwt.clone(),
                    claims: u.claims.clone(),
                }),
            },
            result,
        );
    }

    fn handle_error(ctx: ExportErrorCtx, error: ErrorCode) {
        // Forward to downstream handler
        downstream::handle_error(
            &downstream::ErrorCtx {
                id: ctx.id.clone(),
                protocol_version: ctx.protocol_version.clone(),
                session: ctx.session.as_ref().map(|s| downstream::Session {
                    session_id: s.session_id.clone(),
                    store_id: s.store_id.clone(),
                }),
                user: ctx.user.as_ref().map(|u| downstream::Identity {
                    jwt: u.jwt.clone(),
                    claims: u.claims.clone(),
                }),
            },
            &error,
        );
    }
}

fn handle_resources_list(
    req: ListResourcesRequest,
    ctx: ExportRequestCtx,
) -> Result<ServerResult, ErrorCode> {
    // Try to get resources from imported resources interface
    let our_result = match resources::list_resources(
        &downstream::RequestCtx {
            id: ctx.id.clone(),
            protocol_version: ctx.protocol_version.clone(),
            messages: ctx.messages,
            session: ctx.session.as_ref().map(|s| downstream::Session {
                session_id: s.session_id.clone(),
                store_id: s.store_id.clone(),
            }),
            user: ctx.user.as_ref().map(|u| downstream::Identity {
                jwt: u.jwt.clone(),
                claims: u.claims.clone(),
            }),
        },
        &req,
    ) {
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
    match downstream::handle_request(
        &downstream::RequestCtx {
            id: ctx.id.clone(),
            protocol_version: ctx.protocol_version.clone(),
            messages: ctx.messages,
            session: ctx.session.as_ref().map(|s| downstream::Session {
                session_id: s.session_id.clone(),
                store_id: s.store_id.clone(),
            }),
            user: ctx.user.as_ref().map(|u| downstream::Identity {
                jwt: u.jwt.clone(),
                claims: u.claims.clone(),
            }),
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
                    // Neither component nor downstream implements resources - return MethodNotFound
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
                    // We have resources from imported interface, return them despite downstream error
                    Ok(ServerResult::ResourcesList(our))
                }
                None => {
                    // No resources from imported interface, propagate downstream error
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
    // Try reading from imported resources interface first
    match resources::read_resource(
        &downstream::RequestCtx {
            id: ctx.id.clone(),
            protocol_version: ctx.protocol_version.clone(),
            messages: ctx.messages,
            session: ctx.session.as_ref().map(|s| downstream::Session {
                session_id: s.session_id.clone(),
                store_id: s.store_id.clone(),
            }),
            user: ctx.user.as_ref().map(|u| downstream::Identity {
                jwt: u.jwt.clone(),
                claims: u.claims.clone(),
            }),
        },
        &req,
    ) {
        Ok(Some(result)) => {
            // Imported interface handled it - return the result
            Ok(ServerResult::ResourcesRead(result))
        }
        Ok(None) => {
            // Imported interface doesn't handle this URI - try downstream
            let downstream_req = ClientRequest::ResourcesRead(req.clone());
            match downstream::handle_request(
                &downstream::RequestCtx {
                    id: ctx.id.clone(),
                    protocol_version: ctx.protocol_version.clone(),
                    messages: ctx.messages,
                    session: ctx.session.as_ref().map(|s| downstream::Session {
                        session_id: s.session_id.clone(),
                        store_id: s.store_id.clone(),
                    }),
                    user: ctx.user.as_ref().map(|u| downstream::Identity {
                        jwt: u.jwt.clone(),
                        claims: u.claims.clone(),
                    }),
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
            // Imported interface returned an error - propagate it
            Err(e)
        }
    }
}

fn handle_templates_list(
    req: ListResourceTemplatesRequest,
    ctx: ExportRequestCtx,
) -> Result<ServerResult, ErrorCode> {
    // Try to get templates from imported resources interface
    let our_result = match resources::list_resource_templates(
        &downstream::RequestCtx {
            id: ctx.id.clone(),
            protocol_version: ctx.protocol_version.clone(),
            messages: ctx.messages,
            session: ctx.session.as_ref().map(|s| downstream::Session {
                session_id: s.session_id.clone(),
                store_id: s.store_id.clone(),
            }),
            user: ctx.user.as_ref().map(|u| downstream::Identity {
                jwt: u.jwt.clone(),
                claims: u.claims.clone(),
            }),
        },
        &req,
    ) {
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
    match downstream::handle_request(
        &downstream::RequestCtx {
            id: ctx.id.clone(),
            protocol_version: ctx.protocol_version.clone(),
            messages: ctx.messages,
            session: ctx.session.as_ref().map(|s| downstream::Session {
                session_id: s.session_id.clone(),
                store_id: s.store_id.clone(),
            }),
            user: ctx.user.as_ref().map(|u| downstream::Identity {
                jwt: u.jwt.clone(),
                claims: u.claims.clone(),
            }),
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
                    // Neither component nor downstream implements templates - return MethodNotFound
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
                    // We have templates from imported interface, return them despite downstream error
                    Ok(ServerResult::ResourcesTemplatesList(our))
                }
                None => {
                    // No templates from imported interface, propagate downstream error
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
