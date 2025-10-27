//! MCP resource definitions and management
//!
//! This module organizes resources by category for better maintainability.
//! Each submodule handles a specific type of resource (documentation, agents, WIT, registry).

pub mod agents;
pub mod documentation;
pub mod github;
pub mod registry;
pub mod wit;

use rmcp::ErrorData as McpError;
use rmcp::model::*;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// List all resources from all categories
pub fn list_all(_project_root: &Path) -> Result<ListResourcesResult, McpError> {
    let mut resources = Vec::new();

    // Add documentation resources
    resources.extend(documentation::list());

    // Add agent resources
    resources.extend(agents::list());

    // Add WIT resources
    resources.extend(wit::list());

    // Add registry resources
    resources.extend(registry::list());

    Ok(ListResourcesResult {
        resources,
        next_cursor: None,
    })
}

/// List all resource templates from all categories
pub fn list_templates() -> Result<ListResourceTemplatesResult, McpError> {
    let mut resource_templates = Vec::new();

    // Add documentation templates
    resource_templates.extend(
        documentation::list_templates()
            .into_iter()
            .map(|t| t.no_annotation()),
    );

    // Add agent templates
    resource_templates.extend(
        agents::list_templates()
            .into_iter()
            .map(|t| t.no_annotation()),
    );

    // Add WIT templates
    resource_templates.extend(wit::list_templates().into_iter().map(|t| t.no_annotation()));

    Ok(ListResourceTemplatesResult {
        resource_templates,
        next_cursor: None,
    })
}

/// Read a resource by URI
pub async fn read(
    http_client: &reqwest::Client,
    uri: &str,
    _project_root: &Path,
    local_resources_path: &Option<PathBuf>,
) -> Result<ReadResourceResult, McpError> {
    info!("[READ] read() called with uri: {}", uri);
    info!("[READ] local_resources_path: {:?}", local_resources_path);

    // If local resources path is set, try reading from local filesystem first
    if let Some(repo_root) = local_resources_path {
        info!(
            "[READ] Local resources enabled, using: {}",
            repo_root.display()
        );
        return read_local_resource(uri, repo_root).await;
    }

    info!("[READ] Local resources NOT enabled, using GitHub");

    // Otherwise, use GitHub fetching (existing logic)

    // Route to appropriate module based on URI prefix
    if let Some(resource_name) = uri.strip_prefix("wasmcp://resources/")
        && let Some(result) = documentation::read(http_client, resource_name).await
    {
        return result;
    }

    if let Some(path) = uri.strip_prefix("wasmcp://claude/")
        && let Some(result) = agents::read(http_client, path).await
    {
        return result;
    }

    if let Some(wit_path) = uri.strip_prefix("wasmcp://wit/")
        && let Some(result) = wit::read(http_client, wit_path).await
    {
        return result;
    }

    if let Some(registry_resource) = uri.strip_prefix("wasmcp://registry/")
        && let Some(result) = registry::read(registry_resource).await
    {
        return result;
    }

    // Handle branch-specific resources (templates with {branch} placeholder)
    // Check if URI contains a branch (contains slash after namespace)
    if let Some(namespace) = uri.strip_prefix("wasmcp://") {
        info!("Checking for branch-specific URI, namespace: {}", namespace);
        let has_branch = namespace.starts_with("resources/")
            || namespace.starts_with("claude/")
            || namespace.starts_with("wit/");

        let slash_count = namespace.matches('/').count();
        info!("has_branch: {}, slash_count: {}", has_branch, slash_count);

        // If it's a namespace with potential branch, check if it has branch syntax
        if has_branch && slash_count >= 2 {
            info!("Routing to branch-specific handler");
            // Has format like resources/{branch}/{resource} or wit/{branch}/server/{resource}
            return github::read_branch_resource(http_client, uri).await;
        } else {
            info!(
                "Not branch-specific: has_branch={}, slash_count={}",
                has_branch, slash_count
            );
        }
    }

    warn!("Resource not found: {}", uri);
    Err(McpError::resource_not_found(uri.to_string(), None))
}

/// Read resource from local filesystem
async fn read_local_resource(uri: &str, repo_root: &Path) -> Result<ReadResourceResult, McpError> {
    use std::fs;

    info!("[LOCAL] read_local_resource called with uri: {}", uri);
    info!("[LOCAL] repo_root: {}", repo_root.display());

    info!("[LOCAL] Starting URI to file path mapping");

    // Map URI to local file path
    let file_path = match uri {
        // Documentation resources (standard and branch-specific)
        uri if uri.starts_with("wasmcp://resources/") => {
            info!("[LOCAL] Matched resources URI pattern");
            let remainder = uri.strip_prefix("wasmcp://resources/").unwrap();
            info!("[LOCAL] Resources remainder: {}", remainder);

            // For branch-specific: resources/{branch}/getting-started → getting-started
            let resource_name = if remainder.contains('/') {
                // Has branch - get the last part
                remainder.split('/').next_back().unwrap_or(remainder)
            } else {
                // No branch: resources/getting-started → getting-started
                remainder
            };

            info!("[LOCAL] Extracted resource_name: {}", resource_name);

            match resource_name {
                "getting-started" => repo_root.join("docs/resources/getting-started.md"),
                "building-servers" => repo_root.join("docs/resources/building-servers.md"),
                "registry" => repo_root.join("docs/resources/registry.md"),
                "reference" => repo_root.join("docs/resources/reference.md"),
                "architecture" => repo_root.join("docs/resources/architecture.md"),
                "composition-modes" => repo_root.join("docs/resources/composition-modes.md"),
                _ => {
                    return Err(McpError::resource_not_found(
                        format!("Unknown documentation resource: {}", resource_name),
                        None,
                    ));
                }
            }
        }

        // Agent resources (standard and branch-specific)
        uri if uri.starts_with("wasmcp://claude/") => {
            info!("[LOCAL] Matched claude URI pattern");
            let remainder = uri.strip_prefix("wasmcp://claude/").unwrap();
            // For branch-specific: claude/{branch}/agents/{agent} → agents/{agent}
            let agent_path = if remainder.starts_with("agents/") {
                // No branch: claude/agents/developer → agents/developer
                remainder
            } else {
                // Has branch: claude/feat/downstream/agents/developer → agents/developer
                // Skip first part (branch), take everything after first '/'
                remainder
                    .split_once('/')
                    .map(|(_, rest)| rest)
                    .unwrap_or(remainder)
            };

            match agent_path {
                "agents/developer" => repo_root.join("docs/claude/agents/wasmcp-developer.md"),
                "agents/developer-config" => {
                    repo_root.join("docs/claude/agents/wasmcp-developer.json")
                }
                "agents/toolbuilder" => repo_root.join("docs/claude/agents/wasmcp-toolbuilder.md"),
                "agents/toolbuilder-config" => {
                    repo_root.join("docs/claude/agents/wasmcp-toolbuilder.json")
                }
                _ => {
                    return Err(McpError::resource_not_found(
                        format!("Unknown agent resource: {}", agent_path),
                        None,
                    ));
                }
            }
        }

        // WIT resources (standard and branch-specific)
        uri if uri.starts_with("wasmcp://wit/") => {
            info!("[LOCAL] Matched wit URI pattern");
            let remainder = uri.strip_prefix("wasmcp://wit/").unwrap();
            // For branch-specific: wit/{branch}/protocol/{resource} → protocol/{resource}
            let wit_path = if remainder.starts_with("protocol/") || remainder.starts_with("server/")
            {
                // No branch: wit/protocol/mcp → protocol/mcp
                remainder
            } else {
                // Has branch: wit/feat/downstream/protocol/mcp → protocol/mcp
                // Skip first part (branch), take everything after first '/'
                remainder
                    .split_once('/')
                    .map(|(_, rest)| rest)
                    .unwrap_or(remainder)
            };

            match wit_path {
                "protocol/mcp" => repo_root.join("wit/protocol/mcp.wit"),
                "protocol/features" => repo_root.join("wit/protocol/features.wit"),
                "server/handler" => repo_root.join("wit/server/handler.wit"),
                "server/sessions" => repo_root.join("wit/server/sessions.wit"),
                "server/notifications" => repo_root.join("wit/server/notifications.wit"),
                _ => {
                    return Err(McpError::resource_not_found(
                        format!("Unknown WIT resource: {}", wit_path),
                        None,
                    ));
                }
            }
        }

        // Registry resources - always local, unchanged
        uri if uri.starts_with("wasmcp://registry/") => {
            info!("[LOCAL] Matched registry URI pattern");
            let registry_resource = uri.strip_prefix("wasmcp://registry/").unwrap();
            if let Some(result) = registry::read(registry_resource).await {
                return result;
            } else {
                return Err(McpError::resource_not_found(uri.to_string(), None));
            }
        }

        _ => {
            return Err(McpError::resource_not_found(
                format!("Unknown resource URI: {}", uri),
                None,
            ));
        }
    };

    info!("[LOCAL] Resolved to local path: {}", file_path.display());
    info!("[LOCAL] Attempting to read file...");

    // Read the file
    let content = fs::read_to_string(&file_path).map_err(|e| {
        warn!("[LOCAL] Failed to read file: {}", e);
        McpError::internal_error(
            format!(
                "Local resource not found: {} (hint: check --local-resources path setting)",
                file_path.display()
            ),
            Some(serde_json::json!({
                "uri": uri,
                "file_path": file_path.to_string_lossy(),
                "error": e.to_string(),
            })),
        )
    })?;

    info!(
        "[LOCAL] Successfully read {} bytes from local file",
        content.len()
    );
    info!("[LOCAL] Creating ReadResourceResult...");

    let result = ReadResourceResult {
        contents: vec![ResourceContents::text(content, uri.to_string())],
    };

    info!("[LOCAL] Returning result");
    Ok(result)
}
