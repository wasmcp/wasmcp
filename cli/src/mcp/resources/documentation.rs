//! Documentation resources
//!
//! Provides access to wasmcp documentation including getting started guides,
//! building servers, reference materials, composition modes, registry usage,
//! and architecture documentation.

use super::github;
use rmcp::ErrorData as McpError;
use rmcp::model::{
    Annotated, Annotations, RawResource, RawResourceTemplate, ReadResourceResult, Role,
};

/// List all documentation resources
pub fn list() -> Vec<Annotated<RawResource>> {
    vec![
        Annotated {
            raw: RawResource {
                uri: "wasmcp://resources/getting-started".into(),
                name: "Documentation Index".into(),
                mime_type: Some("text/markdown".into()),
                title: None,
                description: Some("Navigation index mapping questions to resources. Answers: which resource to read, where to start with wasmcp. Contains: topic→resource routing, common question→doc mappings. Skip: if user asks specific topic (building/registry/composition), go directly to that resource.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::User, Role::Assistant]),
                priority: Some(0.9),
                last_modified: None,
            }),
        },
        Annotated {
            raw: RawResource {
                uri: "wasmcp://resources/building-servers".into(),
                name: "Server Development Workflow".into(),
                mime_type: Some("text/markdown".into()),
                title: None,
                description: Some("Server development workflow from creation to execution. Answers: how to create/build/compose/run servers. Contains: wasmcp new commands, build procedures, composition methods (paths/OCI/aliases), wasmtime execution. Skip: for command flags, see 'reference'.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::User, Role::Assistant]),
                priority: Some(0.85),
                last_modified: None,
            }),
        },
        Annotated {
            raw: RawResource {
                uri: "wasmcp://resources/registry".into(),
                name: "Registry Management".into(),
                mime_type: Some("text/markdown".into()),
                title: None,
                description: Some("Component alias and composition profile management. Answers: what are aliases/profiles, how to register components, how to save compositions. Contains: wasmcp registry commands, config file structure (~/.config/wasmcp/config.toml). Skip: for current registered data, see 'registry/components' or 'registry/profiles'.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::User, Role::Assistant]),
                priority: Some(0.65),
                last_modified: None,
            }),
        },
        Annotated {
            raw: RawResource {
                uri: "wasmcp://resources/reference".into(),
                name: "CLI Command Reference".into(),
                mime_type: Some("text/markdown".into()),
                title: None,
                description: Some("CLI command flags and syntax reference. Answers: what flags exist, command options, format specifications. Contains: wasmcp command flags, component spec formats (path/OCI/alias), template types, config file syntax. Skip: for workflows, see 'building-servers'.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::User, Role::Assistant]),
                priority: Some(0.75),
                last_modified: None,
            }),
        },
        Annotated {
            raw: RawResource {
                uri: "wasmcp://resources/architecture".into(),
                name: "Architecture Overview".into(),
                mime_type: Some("text/markdown".into()),
                title: None,
                description: Some("Conceptual overview of wasmcp internals. Answers: how wasmcp works, why use components, design decisions. Contains: capability/middleware pattern, composition pipeline, handler interfaces, component model. Skip: for practical workflow, see 'building-servers'.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::User, Role::Assistant]),
                priority: Some(0.5),
                last_modified: None,
            }),
        },
        Annotated {
            raw: RawResource {
                uri: "wasmcp://resources/composition-modes".into(),
                name: "Composition Modes Guide".into(),
                mime_type: Some("text/markdown".into()),
                title: None,
                description: Some("Comparison of 'compose server' vs 'compose handler' modes. Answers: when to use server vs handler mode, how components layer, what auto-wrapping does. Contains: mode differences, interface layering, real-world examples. Skip: for basic workflow, see 'building-servers'.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::User, Role::Assistant]),
                priority: Some(0.7),
                last_modified: None,
            }),
        },
    ]
}

/// Read a documentation resource by resource name
pub async fn read(
    client: &reqwest::Client,
    resource_name: &str,
) -> Option<Result<ReadResourceResult, McpError>> {
    let path = match resource_name {
        "getting-started" => "docs/resources/getting-started.md",
        "building-servers" => "docs/resources/building-servers.md",
        "registry" => "docs/resources/registry.md",
        "reference" => "docs/resources/reference.md",
        "architecture" => "docs/resources/architecture.md",
        "composition-modes" => "docs/resources/composition-modes.md",
        _ => return None,
    };

    Some(github::fetch_github_file(client, github::default_branch(), path).await)
}

/// Get resource templates for branch-specific access
pub fn list_templates() -> Vec<RawResourceTemplate> {
    vec![RawResourceTemplate {
        uri_template: "wasmcp://resources/{branch}/{resource}".into(),
        name: "Branch-specific Documentation".into(),
        title: None,
        description: Some(
            "Access documentation from specific Git branches (e.g., main, develop, feat/my-feature). Available resources: getting-started, building-servers, registry, reference, architecture, composition-modes"
                .into(),
        ),
        mime_type: Some("text/markdown".into()),
    }]
}
