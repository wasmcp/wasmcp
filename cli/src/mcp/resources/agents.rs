//! Claude agent configuration resources
//!
//! Provides access to agent configuration files for both interactive usage
//! (.md files for ~/.claude/agents/) and headless usage (.json files for --agents flag).

use super::github;
use rmcp::ErrorData as McpError;
use rmcp::model::{
    Annotated, Annotations, RawResource, RawResourceTemplate, ReadResourceResult, Role,
};

/// List all agent configuration resources
pub fn list() -> Vec<Annotated<RawResource>> {
    vec![
        Annotated {
            raw: RawResource {
                uri: "wasmcp://claude/agents/developer".into(),
                name: "CLI Developer Agent".into(),
                mime_type: Some("text/markdown".into()),
                title: None,
                description: Some("Agent configuration for wasmcp CLI internals and development. Answers: how to modify CLI source, debug composition pipeline, add MCP tools/resources, understand wac-graph integration. Contains: CLI architecture, dev-server.sh workflow, MCP server implementation, testing procedures. Skip: for building tools WITH wasmcp, see 'toolbuilder' agent.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::User]),
                priority: Some(0.4),
                last_modified: None,
            }),
        },
        Annotated {
            raw: RawResource {
                uri: "wasmcp://claude/agents/developer-config".into(),
                name: "CLI Developer Agent (JSON)".into(),
                mime_type: Some("application/json".into()),
                title: None,
                description: Some("JSON configuration for CLI developer agent. Answers: same as markdown version. Format: JSON for headless Claude (claude --agents). Skip: for markdown installation format, see 'developer'.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::User]),
                priority: Some(0.4),
                last_modified: None,
            }),
        },
        Annotated {
            raw: RawResource {
                uri: "wasmcp://claude/agents/toolbuilder".into(),
                name: "Tool Builder Agent".into(),
                mime_type: Some("text/markdown".into()),
                title: None,
                description: Some("Agent configuration for building MCP tools with wasmcp. Answers: how to create components, compose servers, debug composition, choose server vs handler mode, test locally. Contains: component creation workflow, composition patterns, debugging guides. Skip: for CLI development, see 'developer' agent.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::User]),
                priority: Some(0.7),
                last_modified: None,
            }),
        },
        Annotated {
            raw: RawResource {
                uri: "wasmcp://claude/agents/toolbuilder-config".into(),
                name: "Tool Builder Agent (JSON)".into(),
                mime_type: Some("application/json".into()),
                title: None,
                description: Some("JSON configuration for tool builder agent. Answers: same as markdown version. Format: JSON for headless Claude (claude --agents). Skip: for markdown installation format, see 'toolbuilder'.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::User]),
                priority: Some(0.7),
                last_modified: None,
            }),
        },
    ]
}

/// Read an agent configuration resource by agent path
pub async fn read(
    client: &reqwest::Client,
    agent_path: &str,
) -> Option<Result<ReadResourceResult, McpError>> {
    let path = match agent_path {
        "agents/developer" => "docs/claude/agents/wasmcp-developer.md",
        "agents/developer-config" => "docs/claude/agents/wasmcp-developer.json",
        "agents/toolbuilder" => "docs/claude/agents/wasmcp-toolbuilder.md",
        "agents/toolbuilder-config" => "docs/claude/agents/wasmcp-toolbuilder.json",
        _ => return None,
    };

    Some(github::fetch_github_file(client, github::default_branch(), path).await)
}

/// Get resource templates for branch-specific access
pub fn list_templates() -> Vec<RawResourceTemplate> {
    vec![RawResourceTemplate {
        uri_template: "wasmcp://claude/{branch}/agents/{agent}".into(),
        name: "Branch-specific Claude Agents".into(),
        title: None,
        description: Some(
            "Access Claude agent configurations from specific Git branches. Available agents: developer, toolbuilder"
                .into(),
        ),
        mime_type: Some("text/markdown".into()),
    }]
}
