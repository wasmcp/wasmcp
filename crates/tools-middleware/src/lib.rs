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

use bindings::exports::wasmcp::mcp::server_handler::Guest;
use bindings::wasmcp::mcp::protocol::*;
use bindings::wasmcp::mcp::server_handler as downstream;
use bindings::wasmcp::mcp::tools_capability as capability;

struct ToolsMiddleware;

impl Guest for ToolsMiddleware {
    fn handle_request(
        id: RequestId,
        request: ClientRequest,
        client: ClientContext,
    ) -> Result<ServerResponse, ErrorCode> {
        match request {
            ClientRequest::ToolsList(req) => handle_tools_list(req, &id, &client),
            ClientRequest::ToolsCall(req) => handle_tools_call(req, &id, &client),
            _ => {
                // Delegate all other requests to downstream handler
                downstream::handle_request(&id, &request, &client)
            }
        }
    }

    fn handle_notification(notification: ClientNotification) {
        // Forward to downstream handler
        downstream::handle_notification(&notification);
    }

    fn handle_response(id: Option<RequestId>, response: Result<ClientResponse, ErrorCode>) {
        // Forward to downstream handler
        downstream::handle_response(id.as_ref(), response.as_ref());
    }
}

fn handle_tools_list(
    req: ListToolsRequest,
    id: &RequestId,
    client: &ClientContext,
) -> Result<ServerResponse, ErrorCode> {
    // Call the imported capability to get tools
    let our_result = capability::list_tools(&req, client);

    // Try to get downstream tools
    match downstream::handle_request(id, &ClientRequest::ToolsList(req.clone()), client) {
        Ok(ServerResponse::ToolsList(downstream_result)) => {
            // Merge our tools with downstream tools
            let mut all_tools = our_result.tools;
            all_tools.extend(downstream_result.tools);

            Ok(ServerResponse::ToolsList(ListToolsResult {
                tools: all_tools,
                next_cursor: downstream_result.next_cursor.or(our_result.next_cursor),
                meta: our_result.meta.or(downstream_result.meta),
            }))
        }
        Err(_) => {
            // Downstream doesn't support tools, just return ours
            Ok(ServerResponse::ToolsList(our_result))
        }
        Ok(_) => {
            // Unexpected response type, just return our tools
            Ok(ServerResponse::ToolsList(our_result))
        }
    }
}

fn handle_tools_call(
    req: CallToolRequest,
    id: &RequestId,
    client: &ClientContext,
) -> Result<ServerResponse, ErrorCode> {
    // Try calling our capability first
    match capability::call_tool(&req, client) {
        Some(result) => {
            // Capability handled it (success or error) - return the result
            Ok(ServerResponse::ToolsCall(result))
        }
        None => {
            // Capability doesn't handle this tool - try downstream
            match downstream::handle_request(id, &ClientRequest::ToolsCall(req.clone()), client) {
                Ok(response) => Ok(response),
                Err(ErrorCode::MethodNotFound(_)) => {
                    // Downstream also doesn't handle it - return InvalidParams
                    // The method exists, but the tool name parameter is invalid
                    Err(ErrorCode::InvalidParams(Error {
                        id: Some(id.clone()),
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
