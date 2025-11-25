use crate::bindings::exports::wasmcp::mcp_v20250618::server_handler::MessageContext;
use crate::bindings::wasmcp::mcp_v20250618::mcp::*;
use crate::bindings::wasmcp::mcp_v20250618::server_handler as downstream;

/// Convert exported MessageContext to imported MessageContext
pub fn to_downstream_ctx<'a>(ctx: &'a MessageContext<'a>) -> downstream::MessageContext<'a> {
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
        http_context: ctx.http_context.clone(),
    }
}

/// Extract HTTP path from message context
pub fn extract_path(ctx: &MessageContext) -> String {
    ctx.http_context
        .as_ref()
        .map(|h| h.path.clone())
        .unwrap_or_else(|| "/mcp".to_string())
}

/// Delegate a request to downstream handler
pub fn delegate_to_downstream(
    ctx: &MessageContext,
    request_id: RequestId,
    request: ClientRequest,
) -> Option<Result<ServerResult, ErrorCode>> {
    let downstream_msg = ClientMessage::Request((request_id, request));
    downstream::handle(&to_downstream_ctx(ctx), downstream_msg)
}

/// Fetch tools list from downstream handler
pub fn fetch_tools_from_downstream(
    ctx: &MessageContext,
    request_id: RequestId,
    req: ListToolsRequest,
) -> Result<Vec<Tool>, ErrorCode> {
    let downstream_req = ClientRequest::ToolsList(req);
    let downstream_msg = ClientMessage::Request((request_id, downstream_req));

    match downstream::handle(&to_downstream_ctx(ctx), downstream_msg) {
        Some(Ok(ServerResult::ToolsList(result))) => Ok(result.tools),
        Some(Ok(_)) => Err(ErrorCode::InternalError(Error {
            code: -32603,
            message: "Unexpected result type from downstream".to_string(),
            data: None,
        })),
        Some(Err(e)) => Err(e),
        None => Err(ErrorCode::MethodNotFound(Error {
            code: -32601,
            message: "Method not found: tools/list".to_string(),
            data: None,
        })),
    }
}