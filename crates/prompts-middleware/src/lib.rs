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

use bindings::exports::wasmcp::server::handler::Guest;
use bindings::wasmcp::server::notifications::NotificationChannel;
use bindings::wasmcp::protocol::mcp::*;
use bindings::wasmcp::server::prompts as capability;
use bindings::wasmcp::server::server_messages::Context;
use bindings::wasmcp::server::handler as downstream;

struct PromptsMiddleware;

impl Guest for PromptsMiddleware {
    fn handle_request(
        ctx: Context,
        request: (ClientRequest, RequestId),
        channel: Option<&NotificationChannel>,
    ) -> Result<ServerResponse, ErrorCode> {
        let (req, id) = request;
        match req {
            ClientRequest::PromptsList(list_req) => {
                handle_prompts_list(list_req, id, &ctx, channel)
            }
            ClientRequest::PromptsGet(get_req) => {
                handle_prompt_get(get_req, id, &ctx, channel)
            }
            _ => {
                // Delegate all other requests to downstream handler
                downstream::handle_request(&ctx, (&req, &id), channel)
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

fn handle_prompts_list(
    req: ListPromptsRequest,
    id: RequestId,
    ctx: &Context,
    channel: Option<&NotificationChannel>,
) -> Result<ServerResponse, ErrorCode> {
    use bindings::wasmcp::protocol::mcp::ListPromptsResult;

    // Try to get prompts from our capability
    let our_result = match capability::list_prompts(ctx, &req, channel) {
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
    match downstream::handle_request(ctx, (&downstream_req, &id), channel) {
        Ok(ServerResponse::PromptsList(downstream_result)) => {
            // Merge our prompts with downstream prompts
            match our_result {
                Some(our) => {
                    let mut all_prompts = our.prompts;
                    all_prompts.extend(downstream_result.prompts);

                    Ok(ServerResponse::PromptsList(ListPromptsResult {
                        prompts: all_prompts,
                        next_cursor: downstream_result.next_cursor.or(our.next_cursor),
                        meta: our.meta.or(downstream_result.meta),
                    }))
                }
                None => {
                    // Only downstream has prompts
                    Ok(ServerResponse::PromptsList(downstream_result))
                }
            }
        }
        Err(ErrorCode::MethodNotFound(_)) => {
            // Downstream doesn't support prompts
            match our_result {
                Some(our) => Ok(ServerResponse::PromptsList(our)),
                None => {
                    // Neither capability nor downstream implements prompts - return MethodNotFound
                    Err(ErrorCode::MethodNotFound(Error {
                        id: Some(id),
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
                    Ok(ServerResponse::PromptsList(our))
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
                Some(our) => Ok(ServerResponse::PromptsList(our)),
                None => {
                    // No prompts available
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

fn handle_prompt_get(
    req: GetPromptRequest,
    id: RequestId,
    ctx: &Context,
    channel: Option<&NotificationChannel>,
) -> Result<ServerResponse, ErrorCode> {
    // Try getting from our capability first
    match capability::get_prompt(ctx, &req, channel) {
        Some(result) => {
            // Capability handled it - return the result
            Ok(ServerResponse::PromptsGet(result))
        }
        None => {
            // Capability doesn't handle this prompt - try downstream
            let downstream_req = ClientRequest::PromptsGet(req.clone());
            match downstream::handle_request(ctx, (&downstream_req, &id), channel) {
                Ok(response) => Ok(response),
                Err(ErrorCode::MethodNotFound(_)) => {
                    // Downstream also doesn't handle it - return InvalidParams
                    // The method exists, but the prompt name parameter is invalid/unknown
                    Err(ErrorCode::InvalidParams(Error {
                        id: Some(id),
                        code: -32602,
                        message: format!("Unknown prompt: {}", req.name),
                        data: None,
                    }))
                }
                Err(e) => Err(e),
            }
        }
    }
}

bindings::export!(PromptsMiddleware with_types_in bindings);
