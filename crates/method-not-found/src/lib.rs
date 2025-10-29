//! Method Not Found Terminal Handler
//!
//! A terminal handler component that returns MethodNotFound errors for all requests.
//! This component sits at the end of a middleware chain as a catch-all.

#![allow(warnings)]

mod bindings {
    wit_bindgen::generate!({
        world: "method-not-found",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp_v20250618::server_handler::{
    ErrorCtx, Guest, NotificationCtx, RequestCtx, ResultCtx,
};
use bindings::wasmcp::mcp_v20250618::mcp;

struct MethodNotFoundHandler;

impl Guest for MethodNotFoundHandler {
    fn handle_request(
        _ctx: RequestCtx,
        request: mcp::ClientRequest,
    ) -> Result<mcp::ServerResult, mcp::ErrorCode> {
        // Determine method name from request variant
        let method = match &request {
            mcp::ClientRequest::Initialize(_) => "initialize",
            mcp::ClientRequest::ToolsList(_) => "tools/list",
            mcp::ClientRequest::ToolsCall(_) => "tools/call",
            mcp::ClientRequest::ResourcesList(_) => "resources/list",
            mcp::ClientRequest::ResourcesRead(_) => "resources/read",
            mcp::ClientRequest::ResourcesTemplatesList(_) => "resources/templates/list",
            mcp::ClientRequest::PromptsList(_) => "prompts/list",
            mcp::ClientRequest::PromptsGet(_) => "prompts/get",
            mcp::ClientRequest::CompletionComplete(_) => "completion/complete",
            mcp::ClientRequest::LoggingSetLevel(_) => "logging/setLevel",
            mcp::ClientRequest::Ping(_) => "ping",
            mcp::ClientRequest::ResourcesSubscribe(_) => "resources/subscribe",
            mcp::ClientRequest::ResourcesUnsubscribe(_) => "resources/unsubscribe",
        };

        // Return MethodNotFound for all requests
        Err(mcp::ErrorCode::MethodNotFound(mcp::Error {
            code: -32601,
            message: format!("Method not found: {}", method),
            data: None,
        }))
    }

    fn handle_notification(_ctx: NotificationCtx, _notification: mcp::ClientNotification) {
        // Terminal handler - silently ignore notifications
        // No downstream to forward to, and notifications don't require responses
    }

    fn handle_result(_ctx: ResultCtx, _result: mcp::ClientResult) {
        // Terminal handler - silently ignore results
        // These are responses from client to server, not common in typical flows
    }

    fn handle_error(_ctx: ErrorCtx, _error: mcp::ErrorCode) {
        // Terminal handler - silently ignore errors
        // These are error responses from client to server
    }
}

bindings::export!(MethodNotFoundHandler with_types_in bindings);
