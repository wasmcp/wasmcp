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

use bindings::exports::wasmcp::server::handler::{Guest, NotificationChannel};
use bindings::wasmcp::server::server_messages::Context;
use bindings::wasmcp::protocol::mcp::{ClientRequest, RequestId, ServerResponse, ErrorCode};

struct MethodNotFoundHandler;

impl Guest for MethodNotFoundHandler {
    fn handle_request(
        _ctx: Context,
        request: (ClientRequest, RequestId),
        _channel: Option<&NotificationChannel>,
    ) -> Result<ServerResponse, ErrorCode> {
        use bindings::wasmcp::protocol::mcp::{ClientRequest, Error, ErrorCode};

        let req = request.0;
        let id = request.1;

        // Determine method name from request variant
        let method = match req {
            ClientRequest::Initialize(_) => "initialize",
            ClientRequest::ToolsList(_) => "tools/list",
            ClientRequest::ToolsCall(_) => "tools/call",
            ClientRequest::ResourcesList(_) => "resources/list",
            ClientRequest::ResourcesRead(_) => "resources/read",
            ClientRequest::ResourcesTemplatesList(_) => "resources/templates/list",
            ClientRequest::PromptsList(_) => "prompts/list",
            ClientRequest::PromptsGet(_) => "prompts/get",
            ClientRequest::CompletionComplete(_) => "completion/complete",
            ClientRequest::LoggingSetLevel(_) => "logging/setLevel",
            ClientRequest::Ping(_) => "ping",
            ClientRequest::ResourcesSubscribe(_) => "resources/subscribe",
            ClientRequest::ResourcesUnsubscribe(_) => "resources/unsubscribe",
        };

        // Return MethodNotFound for all requests
        Err(ErrorCode::MethodNotFound(Error {
            id: Some(id),
            code: -32601,
            message: format!("Method not found: {}", method),
            data: None,
        }))
    }

    fn handle_notification(
        _ctx: Context,
        _notification: bindings::wasmcp::protocol::mcp::ClientNotification,
    ) {
        // Terminal handler - silently ignore notifications
        // No downstream to forward to, and notifications don't require responses
    }

    fn handle_response(
        _ctx: Context,
        _response: Result<
            (
                bindings::wasmcp::protocol::mcp::ClientResponse,
                bindings::wasmcp::protocol::mcp::RequestId,
            ),
            ErrorCode,
        >,
    ) {
        // Terminal handler - silently ignore responses
        // These are responses from client to server, not common in typical flows
    }
}

bindings::export!(MethodNotFoundHandler with_types_in bindings);
