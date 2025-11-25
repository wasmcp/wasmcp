mod bindings {
    wit_bindgen::generate!({
        world: "filter-middleware",
        generate_all,
    });
}

mod config;
mod diagnostic;
mod filtering;
mod helpers;
mod metadata;
mod session;
mod types;

use bindings::exports::wasmcp::mcp_v20250618::server_handler::{Guest, MessageContext};
use bindings::wasmcp::mcp_v20250618::mcp::*;
use bindings::wasmcp::mcp_v20250618::server_handler as downstream;

use config::load_and_aggregate_configs;
use diagnostic::{create_inspect_routing_tool, handle_inspect_routing};
use filtering::FilteringPipeline;
use helpers::{delegate_to_downstream, extract_path, fetch_tools_from_downstream};
use session::{load_tool_registry, store_tool_registry};

struct FilterMiddleware;

impl Guest for FilterMiddleware {
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
                        return delegate_to_downstream(&ctx, request_id, request);
                    }
                };
                Some(result)
            }
            _ => {
                // Forward notifications, results, errors to downstream
                downstream::handle(&helpers::to_downstream_ctx(&ctx), message)
            }
        }
    }
}

/// Handle tools/list - apply path and tag filtering with optimizations
fn handle_tools_list(
    request_id: RequestId,
    req: ListToolsRequest,
    ctx: &MessageContext,
) -> Result<ServerResult, ErrorCode> {
    // Fetch all tools from downstream
    let all_tools = fetch_tools_from_downstream(ctx, request_id, req)?;

    // Load and aggregate routing configs
    let config = match load_and_aggregate_configs(ctx) {
        Ok(c) => c,
        Err(e) => {
            return Err(ErrorCode::InternalError(Error {
                code: -32603,
                message: format!("Failed to load routing configs: {}", e),
                data: None,
            }));
        }
    };

    // Extract current path
    let current_path = extract_path(ctx);

    // Use optimized filtering pipeline
    let pipeline = FilteringPipeline::new(&config, current_path);
    let mut filtered_tools = pipeline.apply_filters(&all_tools);

    // Inject inspect_routing diagnostic tool
    filtered_tools.push(create_inspect_routing_tool());

    // Store filtered tool names in session for validation in tools/call
    if let Err(e) = store_tool_registry(ctx, &filtered_tools) {
        return Err(ErrorCode::InternalError(Error {
            code: -32603,
            message: format!("Failed to store tool registry: {}", e),
            data: None,
        }));
    }

    // Return filtered list with diagnostic tool
    Ok(ServerResult::ToolsList(ListToolsResult {
        tools: filtered_tools,
        next_cursor: None,
        meta: None,
    }))
}

/// Handle tools/call - validate tool is in allowed list
fn handle_tools_call(
    request_id: RequestId,
    req: CallToolRequest,
    ctx: &MessageContext,
) -> Result<ServerResult, ErrorCode> {
    // Check if calling inspect_routing diagnostic tool
    if req.name == "inspect_routing" {
        return handle_inspect_routing(ctx);
    }

    // Load allowed tools from session
    let allowed_tools = match load_tool_registry(ctx) {
        Ok(tools) => tools,
        Err(_) => {
            // If no registry in session, allow call (tools/list may not have been called)
            return delegate_to_downstream(ctx, request_id, ClientRequest::ToolsCall(req))
                .unwrap_or_else(|| {
                    Err(ErrorCode::MethodNotFound(Error {
                        code: -32601,
                        message: "Method not found".to_string(),
                        data: None,
                    }))
                });
        }
    };

    // Validate tool is allowed
    if !allowed_tools.contains(&req.name) {
        return Err(ErrorCode::InvalidParams(Error {
            code: -32602,
            message: format!("Tool '{}' not available at this path", req.name),
            data: None,
        }));
    }

    // Tool is allowed, delegate to downstream
    delegate_to_downstream(ctx, request_id, ClientRequest::ToolsCall(req)).unwrap_or_else(|| {
        Err(ErrorCode::MethodNotFound(Error {
            code: -32601,
            message: "Method not found".to_string(),
            data: None,
        }))
    })
}

bindings::export!(FilterMiddleware with_types_in bindings);