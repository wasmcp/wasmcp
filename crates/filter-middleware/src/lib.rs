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

// JSON-RPC 2.0 error codes
const JSONRPC_INTERNAL_ERROR: i64 = -32603;
const JSONRPC_INVALID_PARAMS: i64 = -32602;
const JSONRPC_METHOD_NOT_FOUND: i64 = -32601;

// Internal request ID for middleware's own requests
const INTERNAL_REQUEST_ID_VALUE: i64 = 0;

use bindings::exports::wasmcp::mcp_v20250618::server_handler::{Guest, MessageContext};
use bindings::wasmcp::mcp_v20250618::mcp::*;
use bindings::wasmcp::mcp_v20250618::server_handler as downstream;

use config::load_and_aggregate_configs;
use diagnostic::{create_inspect_routing_tool, handle_inspect_routing};
use filtering::FilteringPipeline;
use helpers::{delegate_to_downstream, extract_path, fetch_tools_from_downstream};
use session::{load_tool_registry, store_tool_registry};

/// Create a JSON-RPC error response with the given code and message
fn create_error(code: i64, message: String) -> ErrorCode {
    match code {
        JSONRPC_INTERNAL_ERROR => ErrorCode::InternalError(Error {
            code,
            message,
            data: None,
        }),
        JSONRPC_INVALID_PARAMS => ErrorCode::InvalidParams(Error {
            code,
            message,
            data: None,
        }),
        JSONRPC_METHOD_NOT_FOUND => ErrorCode::MethodNotFound(Error {
            code,
            message,
            data: None,
        }),
        _ => ErrorCode::InternalError(Error {
            code,
            message,
            data: None,
        }),
    }
}

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
            return Err(create_error(
                JSONRPC_INTERNAL_ERROR,
                format!("Failed to load routing configs: {}", e),
            ));
        }
    };

    // Extract current path
    let current_path = extract_path(ctx);

    // Use optimized filtering pipeline with copy-on-write optimization
    let pipeline = FilteringPipeline::new(&config, current_path);
    let mut filtered_tools = pipeline.apply_filters(&all_tools).into_owned();

    // Inject inspect_routing diagnostic tool
    filtered_tools.push(create_inspect_routing_tool());

    // Store filtered tool names in session for validation in tools/call
    if let Err(e) = store_tool_registry(ctx, &filtered_tools) {
        return Err(create_error(
            JSONRPC_INTERNAL_ERROR,
            format!("Failed to store tool registry: {}", e),
        ));
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
                    Err(create_error(
                        JSONRPC_METHOD_NOT_FOUND,
                        "Method not found".to_string(),
                    ))
                });
        }
    };

    // Validate tool is allowed
    if !allowed_tools.contains(&req.name) {
        return Err(create_error(
            JSONRPC_INVALID_PARAMS,
            format!("Tool '{}' not available at this path", req.name),
        ));
    }

    // Tool is allowed, delegate to downstream
    delegate_to_downstream(ctx, request_id, ClientRequest::ToolsCall(req)).unwrap_or_else(|| {
        Err(create_error(
            JSONRPC_METHOD_NOT_FOUND,
            "Method not found".to_string(),
        ))
    })
}

bindings::export!(FilterMiddleware with_types_in bindings);
