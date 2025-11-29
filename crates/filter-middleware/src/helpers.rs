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

/// Extract and normalize HTTP path from message context.
///
/// Normalization removes duplicate slashes and trailing slashes to prevent
/// filter bypasses via path manipulation (e.g., "//mcp", "/mcp//").
pub fn extract_path(ctx: &MessageContext) -> String {
    let raw_path = ctx
        .http_context
        .as_ref()
        .map(|h| h.path.as_str())
        .unwrap_or("/mcp");

    // Normalize: remove duplicate slashes, trailing slash, empty segments
    let normalized = raw_path
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/");

    // Ensure leading slash
    if normalized.is_empty() {
        "/mcp".to_string()
    } else {
        format!("/{}", normalized)
    }
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

/// Normalize path by removing duplicate/trailing slashes (internal helper for testing)
#[cfg(test)]
fn normalize_path(raw_path: &str) -> String {
    let normalized = raw_path
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/");

    if normalized.is_empty() {
        "/mcp".to_string()
    } else {
        format!("/{}", normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_removes_duplicate_slashes() {
        assert_eq!(normalize_path("//mcp"), "/mcp");
        assert_eq!(normalize_path("/mcp//calculator"), "/mcp/calculator");
        assert_eq!(normalize_path("///mcp///calculator///"), "/mcp/calculator");
    }

    #[test]
    fn test_normalize_path_removes_trailing_slash() {
        assert_eq!(normalize_path("/mcp/"), "/mcp");
        assert_eq!(normalize_path("/mcp/calculator/"), "/mcp/calculator");
    }

    #[test]
    fn test_normalize_path_handles_normal_paths() {
        assert_eq!(normalize_path("/mcp"), "/mcp");
        assert_eq!(normalize_path("/mcp/calculator"), "/mcp/calculator");
        assert_eq!(
            normalize_path("/mcp/calculator/advanced"),
            "/mcp/calculator/advanced"
        );
    }

    #[test]
    fn test_normalize_path_handles_empty_or_root() {
        assert_eq!(normalize_path(""), "/mcp");
        assert_eq!(normalize_path("/"), "/mcp");
        assert_eq!(normalize_path("//"), "/mcp");
    }

    #[test]
    fn test_normalize_path_prevents_bypass() {
        // All these should normalize to the same path
        assert_eq!(normalize_path("/mcp/calculator"), "/mcp/calculator");
        assert_eq!(normalize_path("//mcp/calculator"), "/mcp/calculator");
        assert_eq!(normalize_path("/mcp//calculator"), "/mcp/calculator");
        assert_eq!(normalize_path("/mcp/calculator//"), "/mcp/calculator");
    }
}
