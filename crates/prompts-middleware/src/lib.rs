//! Prompts Middleware Component
//!
//! A reusable middleware that bridges the MCP protocol (server-handler)
//! with the prompts interface. This component:
//! - Detects prompts/list and prompts/get requests
//! - Calls the imported prompts interface functions
//! - Merges results with downstream handlers
//! - Delegates all other requests downstream

#![allow(warnings)]

mod bindings {
    wit_bindgen::generate!({
        world: "prompts-middleware",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp_v20250618::server_handler::{Guest, MessageContext};
use bindings::wasmcp::mcp_v20250618::mcp::*;
use bindings::wasmcp::mcp_v20250618::prompts;
use bindings::wasmcp::mcp_v20250618::server_handler as downstream;

struct PromptsMiddleware;

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
    }
}

impl Guest for PromptsMiddleware {
    fn handle(
        ctx: MessageContext,
        message: ClientMessage,
    ) -> Option<Result<ServerResult, ErrorCode>> {
        match message {
            ClientMessage::Request((request_id, request)) => {
                // Handle requests - match on request type
                let result = match &request {
                    ClientRequest::PromptsList(list_req) => handle_prompts_list(list_req.clone(), &ctx),
                    ClientRequest::PromptsGet(get_req) => handle_prompt_get(get_req.clone(), &ctx),
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

fn handle_prompts_list(
    req: ListPromptsRequest,
    ctx: &MessageContext,
) -> Result<ServerResult, ErrorCode> {
    // Try to get prompts from imported prompts interface
    let our_result = match prompts::list_prompts(&to_downstream_ctx(ctx), &req) {
        Ok(result) => Some(result),
        Err(ErrorCode::MethodNotFound(_)) => {
            // Component doesn't implement prompts interface - skip it
            None
        }
        Err(e) => {
            // Real error (InvalidParams, InternalError, etc.) - return it
            // Don't hide errors by silently falling back to downstream
            return Err(e);
        }
    };

    // Try to get downstream prompts
    let downstream_req = ClientRequest::PromptsList(req.clone());
    let downstream_msg = ClientMessage::Request((RequestId::Number(0), downstream_req));
    match downstream::handle(&to_downstream_ctx(ctx), downstream_msg) {
        Some(Ok(ServerResult::PromptsList(downstream_result))) => {
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
        Some(Err(ErrorCode::MethodNotFound(_))) => {
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
        Some(Err(e)) => {
            // Downstream returned a real error
            match our_result {
                Some(our) => {
                    // We have prompts from imported interface, return them despite downstream error
                    Ok(ServerResult::PromptsList(our))
                }
                None => {
                    // No prompts from imported interface, propagate downstream error
                    Err(e)
                }
            }
        }
        Some(Ok(_)) => {
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
        None => {
            // Downstream returned None (no response for non-request)
            match our_result {
                Some(our) => Ok(ServerResult::PromptsList(our)),
                None => Err(ErrorCode::MethodNotFound(Error {
                    code: -32601,
                    message: "Method not found: prompts/list".to_string(),
                    data: None,
                })),
            }
        }
    }
}

fn handle_prompt_get(
    req: GetPromptRequest,
    ctx: &MessageContext,
) -> Result<ServerResult, ErrorCode> {
    // Try getting from imported prompts interface first
    match prompts::get_prompt(&to_downstream_ctx(ctx), &req) {
        Ok(Some(result)) => {
            // Imported interface handled it - return the result
            Ok(ServerResult::PromptsGet(result))
        }
        Ok(None) => {
            // Imported interface doesn't handle this prompt - try downstream
            let downstream_req = ClientRequest::PromptsGet(req.clone());
            let downstream_msg = ClientMessage::Request((RequestId::Number(0), downstream_req));
            match downstream::handle(&to_downstream_ctx(ctx), downstream_msg) {
                Some(Ok(response)) => Ok(response),
                Some(Err(ErrorCode::MethodNotFound(_))) | None => {
                    // Downstream also doesn't handle it - return InvalidParams
                    // The method exists, but the prompt name parameter is invalid/unknown
                    Err(ErrorCode::InvalidParams(Error {
                        code: -32602,
                        message: format!("Unknown prompt: {}", req.name),
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

bindings::export!(PromptsMiddleware with_types_in bindings);
