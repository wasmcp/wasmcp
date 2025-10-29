//! Prompts Middleware Component
//!
//! A reusable middleware that bridges the MCP protocol (server-handler)
//! with the clean prompts-capability interface. This component:
//! - Detects prompts/list and prompts/get requests
//! - Calls the imported prompts-capability functions
//! - Merges results with downstream handlers
//! - Delegates all other requests downstream

#![allow(warnings)]

mod bindings {
    wit_bindgen::generate!({
        world: "prompts-middleware",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp_v20250618::server_handler::{
    ErrorCtx as ExportErrorCtx, Guest, NotificationCtx as ExportNotificationCtx,
    RequestCtx as ExportRequestCtx, ResultCtx as ExportResultCtx,
};
use bindings::wasi::io::streams::OutputStream;
use bindings::wasmcp::mcp_v20250618::mcp::*;
use bindings::wasmcp::mcp_v20250618::prompts as capability;
use bindings::wasmcp::mcp_v20250618::server_handler as downstream;

struct PromptsMiddleware;

impl Guest for PromptsMiddleware {
    fn handle_request(
        ctx: ExportRequestCtx,
        request: ClientRequest,
    ) -> Result<ServerResult, ErrorCode> {
        match &request {
            ClientRequest::PromptsList(list_req) => handle_prompts_list(list_req.clone(), ctx),
            ClientRequest::PromptsGet(get_req) => handle_prompt_get(get_req.clone(), ctx),
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

fn handle_prompts_list(
    req: ListPromptsRequest,
    ctx: ExportRequestCtx,
) -> Result<ServerResult, ErrorCode> {
    // Try to get prompts from our capability
    let our_result = match capability::list_prompts(
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
            // Capability doesn't implement prompts interface - skip it
            None
        }
        Err(e) => {
            // Real error (InvalidParams, InternalError, etc.) - return it
            // Don't hide capability errors by silently falling back to downstream
            return Err(e);
        }
    };

    // Try to get downstream prompts
    let downstream_req = ClientRequest::PromptsList(req.clone());
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
        Ok(ServerResult::PromptsList(downstream_result)) => {
            // Merge our prompts with downstream prompts
            match our_result {
                Some(our) => {
                    let mut all_prompts = our.prompts;
                    all_prompts.extend(downstream_result.prompts);

                    Ok(ServerResult::PromptsList(ListPromptsResult {
                        prompts: all_prompts,
                        next_cursor: downstream_result.next_cursor.or(our.next_cursor),
                        meta: our.meta.or(downstream_result.meta),
                    }))
                }
                None => {
                    // Only downstream has prompts
                    Ok(ServerResult::PromptsList(downstream_result))
                }
            }
        }
        Err(ErrorCode::MethodNotFound(_)) => {
            // Downstream doesn't support prompts
            match our_result {
                Some(our) => Ok(ServerResult::PromptsList(our)),
                None => {
                    // Neither capability nor downstream implements prompts - return MethodNotFound
                    Err(ErrorCode::MethodNotFound(Error {
                        code: -32601,
                        message: "Method not found: prompts/list".to_string(),
                        data: None,
                    }))
                }
            }
        }
        Err(e) => {
            // Downstream returned a real error
            match our_result {
                Some(our) => {
                    // We have capability prompts, return them despite downstream error
                    Ok(ServerResult::PromptsList(our))
                }
                None => {
                    // No capability prompts, propagate downstream error
                    Err(e)
                }
            }
        }
        Ok(_) => {
            // Unexpected response type from downstream
            match our_result {
                Some(our) => Ok(ServerResult::PromptsList(our)),
                None => {
                    // No prompts available
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

fn handle_prompt_get(
    req: GetPromptRequest,
    ctx: ExportRequestCtx,
) -> Result<ServerResult, ErrorCode> {
    // Try getting from our capability first
    match capability::get_prompt(
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
            Ok(ServerResult::PromptsGet(result))
        }
        Ok(None) => {
            // Capability doesn't handle this prompt - try downstream
            let downstream_req = ClientRequest::PromptsGet(req.clone());
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
                    // The method exists, but the prompt name parameter is invalid/unknown
                    Err(ErrorCode::InvalidParams(Error {
                        code: -32602,
                        message: format!("Unknown prompt: {}", req.name),
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

bindings::export!(PromptsMiddleware with_types_in bindings);
