//! GitHub resource fetching utilities
//!
//! Provides utilities for fetching documentation and agent configuration files
//! from the wasmcp GitHub repository, supporting branch-specific access.

use rmcp::ErrorData as McpError;
use rmcp::model::*;
use tracing::{debug, error, info, instrument};

const GITHUB_REPO: &str = "https://raw.githubusercontent.com/wasmcp/wasmcp";
const DEFAULT_BRANCH: &str = "main";

/// Fetch a file from GitHub raw content
#[instrument(skip(client), fields(branch = %branch, path = %path))]
pub async fn fetch_github_file(
    client: &reqwest::Client,
    branch: &str,
    path: &str,
) -> Result<ReadResourceResult, McpError> {
    let url = format!("{}/{}/{}", GITHUB_REPO, branch, path);
    info!("Fetching from GitHub: {}", url);

    debug!("Sending HTTP GET request");
    let response = client.get(&url).send().await.map_err(|e| {
        error!("HTTP request failed: {}", e);
        McpError::internal_error(
            format!("Failed to fetch from GitHub: {}", e),
            Some(serde_json::json!({
                "url": url,
                "path": path,
                "error": e.to_string(),
            })),
        )
    })?;

    let status = response.status();
    debug!("Received HTTP response with status: {}", status);

    if !status.is_success() {
        error!("Non-success status code: {}", status);
        return Err(McpError::internal_error(
            format!("GitHub returned status {}: {}", status, url),
            Some(serde_json::json!({
                "url": url,
                "status_code": status.as_u16(),
                "status": status.to_string(),
            })),
        ));
    }

    debug!("Reading response body");
    let content = response.text().await.map_err(|e| {
        error!("Failed to read response body: {}", e);
        McpError::internal_error(
            format!("Failed to read response: {}", e),
            Some(serde_json::json!({
                "url": url,
                "error": e.to_string(),
            })),
        )
    })?;

    let content_len = content.len();
    debug!("Successfully read {} bytes from GitHub", content_len);

    let uri_str = format!("wasmcp://docs/{}", path.replace('/', "-"));
    info!(
        "Successfully fetched resource from GitHub, size: {} bytes",
        content_len
    );

    Ok(ReadResourceResult {
        contents: vec![ResourceContents::text(content, uri_str)],
    })
}

/// Parse and fetch a branch-specific resource from template URI
#[instrument(skip(client), fields(uri = %uri))]
pub async fn read_branch_resource(
    client: &reqwest::Client,
    uri: &str,
) -> Result<ReadResourceResult, McpError> {
    info!("Parsing branch resource URI");
    // Parse URI patterns:
    // wasmcp://resources/{branch}/{resource}
    // wasmcp://claude/{branch}/agents/{agent}
    // wasmcp://wit/{branch}/protocol/{resource}
    // wasmcp://wit/{branch}/server/{resource}

    let path = uri.strip_prefix("wasmcp://").ok_or_else(|| {
        error!("Invalid URI prefix");
        McpError::invalid_params(format!("Invalid resource URI: {}", uri), None)
    })?;

    // Split on first '/' to get namespace
    let (namespace, remainder) = path.split_once('/').ok_or_else(|| {
        error!("URI must have namespace");
        McpError::invalid_params(format!("Invalid URI format: {}", uri), None)
    })?;

    debug!("Namespace: {}, Remainder: {}", namespace, remainder);

    let (branch, resource) = match namespace {
        "resources" => {
            // wasmcp://resources/{branch}/{resource}
            let (branch, resource) = remainder.split_once('/').ok_or_else(|| {
                McpError::invalid_params(format!("Invalid resources URI format: {}", uri), None)
            })?;
            (branch, resource.to_string())
        }
        "claude" => {
            // wasmcp://claude/{branch}/agents/{agent}
            let (branch, rest) = remainder.split_once('/').ok_or_else(|| {
                McpError::invalid_params(format!("Invalid claude URI format: {}", uri), None)
            })?;
            (branch, rest.to_string())
        }
        "wit" => {
            // wasmcp://wit/{branch}/protocol/{resource} or wasmcp://wit/{branch}/server/{resource}
            let (branch, rest) = remainder.split_once('/').ok_or_else(|| {
                McpError::invalid_params(format!("Invalid wit URI format: {}", uri), None)
            })?;
            (branch, rest.to_string())
        }
        _ => {
            return Err(McpError::invalid_params(
                format!("Unknown namespace: {}", namespace),
                None,
            ));
        }
    };

    info!(
        "Parsed branch: {}, namespace: {}, resource: {}",
        branch, namespace, resource
    );

    // Validate branch name (basic security check - allow /, -, _, . and alphanumeric)
    if !branch
        .chars()
        .all(|c| c.is_alphanumeric() || c == '/' || c == '.' || c == '-' || c == '_')
    {
        error!("Invalid characters in branch name: {}", branch);
        return Err(McpError::invalid_params(
            format!("Invalid branch name: {}", branch),
            None,
        ));
    }

    // Map namespace and resource to file path
    let file_path = match namespace {
        "resources" => match resource.as_str() {
            "getting-started" => "docs/resources/getting-started.md",
            "building-servers" => "docs/resources/building-servers.md",
            "registry" => "docs/resources/registry.md",
            "reference" => "docs/resources/reference.md",
            "architecture" => "docs/resources/architecture.md",
            "composition-modes" => "docs/resources/composition-modes.md",
            _ => return Err(McpError::resource_not_found(uri.to_string(), None)),
        },
        "claude" => {
            // resource is like "agents/developer" or "agents/toolbuilder"
            let mut claude_parts = resource.split('/');
            let claude_namespace = claude_parts.next().ok_or_else(|| {
                McpError::invalid_params(
                    format!("Claude resource must include namespace: {}", uri),
                    None,
                )
            })?;
            let claude_resource = claude_parts.next().ok_or_else(|| {
                McpError::invalid_params(
                    format!("Claude resource must include resource name: {}", uri),
                    None,
                )
            })?;
            match (claude_namespace, claude_resource) {
                ("agents", "developer") => "docs/claude/agents/wasmcp-developer.md",
                ("agents", "developer-config") => "docs/claude/agents/wasmcp-developer.json",
                ("agents", "toolbuilder") => "docs/claude/agents/wasmcp-toolbuilder.md",
                ("agents", "toolbuilder-config") => "docs/claude/agents/wasmcp-toolbuilder.json",
                _ => return Err(McpError::resource_not_found(uri.to_string(), None)),
            }
        }
        "wit" => {
            // resource is like "protocol/mcp" or "server/handler"
            let mut wit_parts = resource.split('/');
            let wit_namespace = wit_parts.next().ok_or_else(|| {
                McpError::invalid_params(
                    format!("WIT resource must include protocol or server: {}", uri),
                    None,
                )
            })?;
            let wit_resource = wit_parts.next().ok_or_else(|| {
                McpError::invalid_params(
                    format!("WIT resource must include resource name: {}", uri),
                    None,
                )
            })?;
            match (wit_namespace, wit_resource) {
                ("protocol", "mcp") => "wit/protocol/mcp.wit",
                ("protocol", "features") => "wit/protocol/features.wit",
                ("server", "handler") => "wit/server/handler.wit",
                ("server", "sessions") => "wit/server/sessions.wit",
                ("server", "notifications") => "wit/server/notifications.wit",
                _ => return Err(McpError::resource_not_found(uri.to_string(), None)),
            }
        }
        _ => return Err(McpError::resource_not_found(uri.to_string(), None)),
    };

    fetch_github_file(client, branch, file_path).await
}

/// Get default branch name
pub fn default_branch() -> &'static str {
    DEFAULT_BRANCH
}
