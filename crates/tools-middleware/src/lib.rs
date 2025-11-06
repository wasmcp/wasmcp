//! Tools Middleware Component
//!
//! A reusable middleware that bridges the MCP protocol (server-handler)
//! with the tools interface. This component:
//! - Detects tools/list and tools/call requests
//! - Calls the imported tools interface functions
//! - Merges results with downstream handlers
//! - Delegates all other requests downstream

#![allow(warnings)]

mod bindings {
    wit_bindgen::generate!({
        world: "tools-middleware",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp_v20250618::server_handler::{Guest, MessageContext};
use bindings::wasmcp::mcp_v20250618::mcp::*;
use bindings::wasmcp::mcp_v20250618::server_handler as downstream;
use bindings::wasmcp::mcp_v20250618::tools;

struct ToolsMiddleware;

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

impl Guest for ToolsMiddleware {
    fn handle(
        ctx: MessageContext,
        message: ClientMessage,
    ) -> Option<Result<ServerResult, ErrorCode>> {
        match message {
            ClientMessage::Request((request_id, request)) => {
                // Handle requests - match on request type
                let result = match &request {
                    ClientRequest::ToolsList(list_req) => {
                        handle_tools_list(request_id.clone(), list_req.clone(), &ctx)
                    }
                    ClientRequest::ToolsCall(call_req) => {
                        handle_tools_call(request_id.clone(), call_req.clone(), &ctx)
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

fn handle_tools_list(
    request_id: RequestId,
    req: ListToolsRequest,
    ctx: &MessageContext,
) -> Result<ServerResult, ErrorCode> {
    // Try to get tools from imported tools interface
    let our_result = match tools::list_tools(&to_downstream_ctx(ctx), &req) {
        Ok(result) => Some(result),
        Err(ErrorCode::MethodNotFound(_)) => {
            // Component doesn't implement tools interface - skip it
            None
        }
        Err(e) => {
            // Real error (InvalidParams, InternalError, etc.) - return it
            // Don't hide errors by silently falling back to downstream
            return Err(e);
        }
    };

    // Try to get downstream tools - preserve the original request ID
    let downstream_req = ClientRequest::ToolsList(req.clone());
    let downstream_msg = ClientMessage::Request((request_id, downstream_req));
    match downstream::handle(&to_downstream_ctx(ctx), downstream_msg) {
        Some(Ok(ServerResult::ToolsList(downstream_result))) => {
            // Merge our tools with downstream tools
            match our_result {
                Some(our) => {
                    let mut all_tools = our.tools;
                    all_tools.extend(downstream_result.tools);

                    Ok(ServerResult::ToolsList(ListToolsResult {
                        tools: all_tools,
                        next_cursor: downstream_result.next_cursor.or(our.next_cursor),
                        meta: our.meta.or(downstream_result.meta),
                    }))
                }
                None => {
                    // Only downstream has tools
                    Ok(ServerResult::ToolsList(downstream_result))
                }
            }
        }
        Some(Err(ErrorCode::MethodNotFound(_))) => {
            // Downstream doesn't support tools
            match our_result {
                Some(our) => Ok(ServerResult::ToolsList(our)),
                None => {
                    // Neither component nor downstream implements tools - return MethodNotFound
                    Err(ErrorCode::MethodNotFound(Error {
                        code: -32601,
                        message: "Method not found: tools/list".to_string(),
                        data: None,
                    }))
                }
            }
        }
        Some(Err(e)) => {
            // Downstream returned a real error
            match our_result {
                Some(our) => {
                    // We have tools from imported interface, return them despite downstream error
                    Ok(ServerResult::ToolsList(our))
                }
                None => {
                    // No tools from imported interface, propagate downstream error
                    Err(e)
                }
            }
        }
        Some(Ok(_)) => {
            // Unexpected response type from downstream
            match our_result {
                Some(our) => Ok(ServerResult::ToolsList(our)),
                None => {
                    // No tools available
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
                Some(our) => Ok(ServerResult::ToolsList(our)),
                None => Err(ErrorCode::MethodNotFound(Error {
                    code: -32601,
                    message: "Method not found: tools/list".to_string(),
                    data: None,
                })),
            }
        }
    }
}

fn handle_tools_call(
    request_id: RequestId,
    req: CallToolRequest,
    ctx: &MessageContext,
) -> Result<ServerResult, ErrorCode> {
    // Try calling imported tools interface first
    match tools::call_tool(&to_downstream_ctx(ctx), &req) {
        Ok(Some(result)) => {
            // Imported interface handled it - return the result
            Ok(ServerResult::ToolsCall(result))
        }
        Ok(None) => {
            // Imported interface doesn't handle this tool - try downstream
            // Preserve the original request ID
            let downstream_req = ClientRequest::ToolsCall(req.clone());
            let downstream_msg = ClientMessage::Request((request_id, downstream_req));
            match downstream::handle(&to_downstream_ctx(ctx), downstream_msg) {
                Some(Ok(response)) => Ok(response),
                Some(Err(ErrorCode::MethodNotFound(_))) | None => {
                    // Downstream also doesn't handle it - return InvalidParams
                    // The method exists, but the tool name parameter is invalid
                    Err(ErrorCode::InvalidParams(Error {
                        code: -32602,
                        message: format!("Unknown tool: {}", req.name),
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

bindings::export!(ToolsMiddleware with_types_in bindings);
