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
                uri: "wasmcp://wit/mcp".into(),
                name: "MCP Protocol Types".into(),
                mime_type: Some("text/plain".into()),
                title: None,
                description: Some("Complete MCP protocol type definitions including JSON-RPC, requests, responses, errors, and capability interfaces (tools, resources, prompts). Answers: what MCP message types exist, protocol wire format, request/response structure, what capability interfaces exist. Contains: all MCP types and capability interface definitions. Skip: for handler chaining and server interfaces, see 'server'.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::Assistant]),
                priority: Some(0.9),
                last_modified: None,
            }),
        },
        Annotated {
            raw: RawResource {
                uri: "wasmcp://wit/server".into(),
                name: "Server Interfaces".into(),
                mime_type: Some("text/plain".into()),
                title: None,
                description: Some("Server interfaces including handler chaining, notification/messages. Answers: what is server-handler interface, why 'does not export server-handler' error occurs, how components chain together, how to send notifications. Contains: server-handler interface, server-messages interface for notifications. Skip: for protocol message types, see 'mcp'.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::Assistant]),
                priority: Some(0.8),
                last_modified: None,
            }),
        },
        Annotated {
            raw: RawResource {
                uri: "wasmcp://wit/sessions".into(),
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
    ]
}

/// Read a WIT interface resource by wit path (e.g., "mcp", "server", "sessions")
pub async fn read(
    client: &reqwest::Client,
    wit_path: &str,
) -> Option<Result<ReadResourceResult, McpError>> {
    let path = match wit_path {
        "mcp" => "spec/2025-06-18/wit/mcp.wit",
        "server" => "spec/2025-06-18/wit/server.wit",
        "sessions" => "spec/2025-06-18/wit/sessions.wit",
        _ => return None,
    };

    Some(github::fetch_github_file(client, github::default_branch(), path).await)
}

/// Get resource templates for branch-specific access
pub fn list_templates() -> Vec<RawResourceTemplate> {
    vec![
        RawResourceTemplate {
            uri_template: "wasmcp://wit/{branch}/{resource}".into(),
            name: "Branch-specific WIT Interfaces".into(),
            title: None,
            description: Some(
                "Access WIT interfaces from specific Git branches. Available resources: mcp (protocol types and capabilities), server (handler and messages), sessions (state management)"
                    .into(),
            ),
            mime_type: Some("text/plain".into()),
        },
    ]
}
