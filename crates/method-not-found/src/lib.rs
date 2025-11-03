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

use bindings::exports::wasmcp::mcp_v20250618::server_handler::{Guest, MessageContext};
use bindings::wasmcp::mcp_v20250618::mcp;

struct MethodNotFoundHandler;

impl Guest for MethodNotFoundHandler {
    fn handle(
        _ctx: MessageContext,
        message: mcp::ClientMessage,
    ) -> Option<Result<mcp::ServerResult, mcp::ErrorCode>> {
        // Only handle request messages
        let mcp::ClientMessage::Request((_request_id, request)) = message else {
            // Not a request - terminal handler returns None (no response)
            return None;
        };

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
        Some(Err(mcp::ErrorCode::MethodNotFound(mcp::Error {
            code: -32601,
            message: format!("Method not found: {}", method),
            data: None,
        })))
    }
}

bindings::export!(MethodNotFoundHandler with_types_in bindings);
