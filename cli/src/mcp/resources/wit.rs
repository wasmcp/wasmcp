//! WIT interface resources
//!
//! Provides access to WIT (WebAssembly Interface Type) definitions
//! for protocol and server interfaces.

use super::github;
use rmcp::ErrorData as McpError;
use rmcp::model::{
    Annotated, Annotations, RawResource, RawResourceTemplate, ReadResourceResult, Role,
};

/// List all WIT interface resources
pub fn list() -> Vec<Annotated<RawResource>> {
    vec![
        Annotated {
            raw: RawResource {
                uri: "wasmcp://wit/protocol/mcp".into(),
                name: "MCP Protocol Types".into(),
                mime_type: Some("text/plain".into()),
                title: None,
                description: Some("Complete MCP protocol type definitions (JSON-RPC, requests, responses, errors). Answers: what MCP message types exist, protocol wire format, request/response structure. Contains: all MCP types that components handle. Skip: for handler chaining, see 'server/handler'.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::Assistant]),
                priority: Some(0.5),
                last_modified: None,
            }),
        },
        Annotated {
            raw: RawResource {
                uri: "wasmcp://wit/protocol/features".into(),
                name: "MCP Capability Interfaces".into(),
                mime_type: Some("text/plain".into()),
                title: None,
                description: Some("MCP capability interfaces (tools, resources, prompts) that components export. Answers: what capability interfaces exist, what methods components implement. Contains: tools/resources/prompts interface definitions. Skip: for handler interface, see 'server/handler'.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::Assistant]),
                priority: Some(0.4),
                last_modified: None,
            }),
        },
        Annotated {
            raw: RawResource {
                uri: "wasmcp://wit/server/handler".into(),
                name: "Server Handler Interface".into(),
                mime_type: Some("text/plain".into()),
                title: None,
                description: Some("WIT interface that all middleware/transport components must export. Answers: what is server-handler interface, why 'does not export server-handler' error occurs, how components chain together. Contains: interface definition, import/export requirements. Skip: for protocol message types, see 'protocol/mcp'.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::Assistant]),
                priority: Some(0.6),
                last_modified: None,
            }),
        },
        Annotated {
            raw: RawResource {
                uri: "wasmcp://wit/server/sessions".into(),
                name: "Session Management Interface".into(),
                mime_type: Some("text/plain".into()),
                title: None,
                description: Some("Session management interfaces for stateful middleware components. Answers: how to implement session state, WASI KV integration. Contains: session interface definitions. Skip: for basic composition, not needed unless building stateful middleware.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::Assistant]),
                priority: Some(0.3),
                last_modified: None,
            }),
        },
        Annotated {
            raw: RawResource {
                uri: "wasmcp://wit/server/messages".into(),
                name: "Notification Interface".into(),
                mime_type: Some("text/plain".into()),
                title: None,
                description: Some("Server-to-client notification interfaces (progress, logs, resource updates). Answers: how to send messages, what notification types exist. Contains: notification interface definitions. Skip: for basic composition, not needed unless building notification middleware.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::Assistant]),
                priority: Some(0.3),
                last_modified: None,
            }),
        },
    ]
}

/// Read a WIT interface resource by wit path (e.g., "protocol/mcp")
pub async fn read(
    client: &reqwest::Client,
    wit_path: &str,
) -> Option<Result<ReadResourceResult, McpError>> {
    let path = match wit_path {
        "protocol/mcp" => "wit/protocol/mcp.wit",
        "protocol/features" => "wit/protocol/features.wit",
        "server/handler" => "wit/server/handler.wit",
        "server/sessions" => "wit/server/sessions.wit",
        "server/messages" => "wit/server/messages.wit",
        _ => return None,
    };

    Some(github::fetch_github_file(client, github::default_branch(), path).await)
}

/// Get resource templates for branch-specific access
pub fn list_templates() -> Vec<RawResourceTemplate> {
    vec![
        RawResourceTemplate {
            uri_template: "wasmcp://wit/{branch}/protocol/{resource}".into(),
            name: "Branch-specific WIT Protocol Interfaces".into(),
            title: None,
            description: Some(
                "Access WIT protocol interfaces from specific Git branches. Available resources: mcp, features"
                    .into(),
            ),
            mime_type: Some("text/plain".into()),
        },
        RawResourceTemplate {
            uri_template: "wasmcp://wit/{branch}/server/{resource}".into(),
            name: "Branch-specific WIT Server Interfaces".into(),
            title: None,
            description: Some(
                "Access WIT server interfaces from specific Git branches. Available resources: handler, sessions, messages"
                    .into(),
            ),
            mime_type: Some("text/plain".into()),
        },
    ]
}
