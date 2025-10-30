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

use bindings::exports::wasmcp::mcp_v20250618::server_handler::{
    ErrorCtx as ExportErrorCtx, Guest, Identity as ExportIdentity,
    NotificationCtx as ExportNotificationCtx, RequestCtx as ExportRequestCtx,
    ResultCtx as ExportResultCtx, Session as ExportSession,
};
use bindings::wasi::io::streams::OutputStream;
use bindings::wasmcp::mcp_v20250618::mcp::*;
use bindings::wasmcp::mcp_v20250618::server_handler as downstream;
use bindings::wasmcp::mcp_v20250618::tools;

struct ToolsMiddleware;

impl Guest for ToolsMiddleware {
    fn handle_request(
        ctx: ExportRequestCtx,
        request: ClientRequest,
    ) -> Result<ServerResult, ErrorCode> {
        match &request {
            ClientRequest::ToolsList(list_req) => handle_tools_list(list_req.clone(), ctx),
            ClientRequest::ToolsCall(call_req) => handle_tools_call(call_req.clone(), ctx),
            _ => {
                // Delegate all other requests to downstream handler
                downstream::handle_request(
                    &downstream::RequestCtx {
                        id: ctx.id.clone(),
                        protocol_version: ctx.protocol_version.clone(),
                        messages: ctx.messages,
                        session: ctx.session.as_ref().map(|s| downstream::Session {
                            session_id: s.session_id.clone(),
                            store_id: s.store_id.clone(),
                        }),
                        user: ctx.user.as_ref().map(|u| downstream::Identity {
                            jwt: u.jwt.clone(),
                            claims: u.claims.clone(),
                        }),
                    },
                    &request,
                )
            }
        }
    }

    fn handle_notification(ctx: ExportNotificationCtx, notification: ClientNotification) {
        // Forward to downstream handler
        downstream::handle_notification(
            &downstream::NotificationCtx {
                protocol_version: ctx.protocol_version.clone(),
                session: ctx.session.as_ref().map(|s| downstream::Session {
                    session_id: s.session_id.clone(),
                    store_id: s.store_id.clone(),
                }),
                user: ctx.user.as_ref().map(|u| downstream::Identity {
                    jwt: u.jwt.clone(),
                    claims: u.claims.clone(),
                }),
            },
            &notification,
        );
    }

    fn handle_result(ctx: ExportResultCtx, result: ClientResult) {
        // Forward to downstream handler
        downstream::handle_result(
            &downstream::ResultCtx {
                id: ctx.id.clone(),
                protocol_version: ctx.protocol_version.clone(),
                session: ctx.session.as_ref().map(|s| downstream::Session {
                    session_id: s.session_id.clone(),
                    store_id: s.store_id.clone(),
                }),
                user: ctx.user.as_ref().map(|u| downstream::Identity {
                    jwt: u.jwt.clone(),
                    claims: u.claims.clone(),
                }),
            },
            result,
        );
    }

    fn handle_error(ctx: ExportErrorCtx, error: ErrorCode) {
        // Forward to downstream handler
        downstream::handle_error(
            &downstream::ErrorCtx {
                id: ctx.id.clone(),
                protocol_version: ctx.protocol_version.clone(),
                session: ctx.session.as_ref().map(|s| downstream::Session {
                    session_id: s.session_id.clone(),
                    store_id: s.store_id.clone(),
                }),
                user: ctx.user.as_ref().map(|u| downstream::Identity {
                    jwt: u.jwt.clone(),
                    claims: u.claims.clone(),
                }),
            },
            &error,
        );
    }
}

fn handle_tools_list(
    req: ListToolsRequest,
    ctx: ExportRequestCtx,
) -> Result<ServerResult, ErrorCode> {
    // Try to get tools from imported tools interface
    let our_result = match tools::list_tools(
        &downstream::RequestCtx {
            id: ctx.id.clone(),
            protocol_version: ctx.protocol_version.clone(),
            messages: ctx.messages,
            session: ctx.session.as_ref().map(|s| downstream::Session {
                session_id: s.session_id.clone(),
                store_id: s.store_id.clone(),
            }),
            user: ctx.user.as_ref().map(|u| downstream::Identity {
                jwt: u.jwt.clone(),
                claims: u.claims.clone(),
            }),
        },
        &req,
    ) {
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

    // Try to get downstream tools
    let downstream_req = ClientRequest::ToolsList(req.clone());
    match downstream::handle_request(
        &downstream::RequestCtx {
            id: ctx.id.clone(),
            protocol_version: ctx.protocol_version.clone(),
            messages: ctx.messages,
            session: ctx.session.as_ref().map(|s| downstream::Session {
                session_id: s.session_id.clone(),
                store_id: s.store_id.clone(),
            }),
            user: ctx.user.as_ref().map(|u| downstream::Identity {
                jwt: u.jwt.clone(),
                claims: u.claims.clone(),
            }),
        },
        &downstream_req,
    ) {
        Ok(ServerResult::ToolsList(downstream_result)) => {
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
        Err(ErrorCode::MethodNotFound(_)) => {
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
        Err(e) => {
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
        Ok(_) => {
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
    }
}

fn handle_tools_call(
    req: CallToolRequest,
    ctx: ExportRequestCtx,
) -> Result<ServerResult, ErrorCode> {
    // Try calling imported tools interface first
    match tools::call_tool(
        &downstream::RequestCtx {
            id: ctx.id.clone(),
            protocol_version: ctx.protocol_version.clone(),
            messages: ctx.messages,
            session: ctx.session.as_ref().map(|s| downstream::Session {
                session_id: s.session_id.clone(),
                store_id: s.store_id.clone(),
            }),
            user: ctx.user.as_ref().map(|u| downstream::Identity {
                jwt: u.jwt.clone(),
                claims: u.claims.clone(),
            }),
        },
        &req,
    ) {
        Ok(Some(result)) => {
            // Imported interface handled it - return the result
            Ok(ServerResult::ToolsCall(result))
        }
        Ok(None) => {
            // Imported interface doesn't handle this tool - try downstream
            let downstream_req = ClientRequest::ToolsCall(req.clone());
            match downstream::handle_request(
                &downstream::RequestCtx {
                    id: ctx.id.clone(),
                    protocol_version: ctx.protocol_version.clone(),
                    messages: ctx.messages,
                    session: ctx.session.as_ref().map(|s| downstream::Session {
                        session_id: s.session_id.clone(),
                        store_id: s.store_id.clone(),
                    }),
                    user: ctx.user.as_ref().map(|u| downstream::Identity {
                        jwt: u.jwt.clone(),
                        claims: u.claims.clone(),
                    }),
                },
                &downstream_req,
            ) {
                Ok(response) => Ok(response),
                Err(ErrorCode::MethodNotFound(_)) => {
                    // Downstream also doesn't handle it - return InvalidParams
                    // The method exists, but the tool name parameter is invalid
                    Err(ErrorCode::InvalidParams(Error {
                        code: -32602,
                        message: format!("Unknown tool: {}", req.name),
                        data: None,
                    }))
                }
                Err(e) => Err(e),
            }
        }
        Err(e) => {
            // Imported interface returned an error - propagate it
            Err(e)
        }
    }
}

bindings::export!(ToolsMiddleware with_types_in bindings);
