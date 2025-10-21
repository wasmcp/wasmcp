//! Tools Middleware Component
//!
//! A reusable middleware that bridges the MCP protocol (server-handler)
//! with the clean tools-capability interface. This component:
//! - Detects tools/list and tools/call requests
//! - Calls the imported tools-capability functions
//! - Merges results with downstream handlers
//! - Delegates all other requests downstream

#![allow(warnings)]

mod bindings {
    wit_bindgen::generate!({
        world: "tools-middleware",
        generate_all,
    });
}

use bindings::exports::wasmcp::server::handler::Guest;
use bindings::wasmcp::protocol::mcp::*;
use bindings::wasmcp::server::server_messages::Context;
use bindings::wasmcp::server::tools as capability;
use bindings::wasmcp::server::handler as downstream;
use bindings::wasmcp::server::notifications::NotificationChannel;

struct ToolsMiddleware;

impl Guest for ToolsMiddleware {
    fn handle_request(
        ctx: Context,
        request: (ClientRequest, RequestId),
        channel: Option<&NotificationChannel>,
    ) -> Result<ServerResponse, ErrorCode> {
        let (req, id) = request;
        match req {
            ClientRequest::ToolsList(list_req) => {
                handle_tools_list(list_req, id, &ctx, channel)
            }
            ClientRequest::ToolsCall(call_req) => {
                handle_tools_call(call_req, id, &ctx, channel)
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

fn handle_tools_list(
    req: ListToolsRequest,
    id: RequestId,
    ctx: &Context,
    channel: Option<&NotificationChannel>,
) -> Result<ServerResponse, ErrorCode> {
    use bindings::wasmcp::protocol::mcp::ListToolsResult;

    // Try to get tools from our capability
    let our_result = match capability::list_tools(ctx, &req, channel) {
        Ok(result) => Some(result),
        Err(ErrorCode::MethodNotFound(_)) => {
            // Capability doesn't implement tools interface - skip it
            None
        }
        Err(e) => {
            // Real error (InvalidParams, InternalError, etc.) - return it
            // Don't hide capability errors by silently falling back to downstream
            return Err(e);
        }
    };

    // Try to get downstream tools
    let downstream_req = ClientRequest::ToolsList(req.clone());
    match downstream::handle_request(ctx, (&downstream_req, &id), channel) {
        Ok(ServerResponse::ToolsList(downstream_result)) => {
            // Merge our tools with downstream tools
            match our_result {
                Some(our) => {
                    let mut all_tools = our.tools;
                    all_tools.extend(downstream_result.tools);

                    Ok(ServerResponse::ToolsList(ListToolsResult {
                        tools: all_tools,
                        next_cursor: downstream_result.next_cursor.or(our.next_cursor),
                        meta: our.meta.or(downstream_result.meta),
                    }))
                }
                None => {
                    // Only downstream has tools
                    Ok(ServerResponse::ToolsList(downstream_result))
                }
            }
        }
        Err(ErrorCode::MethodNotFound(_)) => {
            // Downstream doesn't support tools
            match our_result {
                Some(our) => Ok(ServerResponse::ToolsList(our)),
                None => {
                    // Neither capability nor downstream implements tools - return MethodNotFound
                    Err(ErrorCode::MethodNotFound(Error {
                        id: Some(id),
                        code: -32601,
                        message: "Method not found: tools/list".to_string(),
                        data: None,
                    }))
                }
            }
        }
        Err(e) => {
            // Downstream returned a real error
            match our_result {
                Some(our) => {
                    // We have capability tools, return them despite downstream error
                    Ok(ServerResponse::ToolsList(our))
                }
                None => {
                    // No capability tools, propagate downstream error
                    Err(e)
                }
            }
        }
        Ok(_) => {
            // Unexpected response type from downstream
            match our_result {
                Some(our) => Ok(ServerResponse::ToolsList(our)),
                None => {
                    // No tools available
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

fn handle_tools_call(
    req: CallToolRequest,
    id: RequestId,
    ctx: &Context,
    channel: Option<&NotificationChannel>,
) -> Result<ServerResponse, ErrorCode> {
    // Try calling our capability first
    match capability::call_tool(ctx, &req, channel) {
        Some(result) => {
            // Capability handled it - return the result
            Ok(ServerResponse::ToolsCall(result))
        }
        None => {
            // Capability doesn't handle this tool - try downstream
            let downstream_req = ClientRequest::ToolsCall(req.clone());
            match downstream::handle_request(ctx, (&downstream_req, &id), channel) {
                Ok(response) => Ok(response),
                Err(ErrorCode::MethodNotFound(_)) => {
                    // Downstream also doesn't handle it - return InvalidParams
                    // The method exists, but the tool name parameter is invalid
                    Err(ErrorCode::InvalidParams(Error {
                        id: Some(id),
                        code: -32602,
                        message: format!("Unknown tool: {}", req.name),
                        data: None,
                    }))
                }
                Err(e) => Err(e),
            }
        }
    }
}

bindings::export!(ToolsMiddleware with_types_in bindings);
