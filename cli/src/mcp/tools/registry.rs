//! Registry management tools
//!
//! Provides MCP tool interfaces for managing the wasmcp registry including
//! component aliases, composition profiles, and configuration.

use rmcp::ErrorData as McpError;
use rmcp::model::*;
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::process::Command;

/// Arguments for listing registry contents
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RegistryListArgs {
    /// What to list (components, profiles, or all)
    #[serde(default = "default_list_target")]
    pub target: String,
}

fn default_list_target() -> String {
    "all".to_string()
}

/// Arguments for adding a component alias
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddComponentArgs {
    /// Component alias name
    pub alias: String,

    /// Component path or reference
    pub spec: String,
}

/// Arguments for adding a composition profile
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddProfileArgs {
    /// Profile name
    pub name: String,

    /// Components in profile
    pub components: Vec<String>,

    /// Output path (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

/// Arguments for removing registry items
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveArgs {
    /// Type to remove (component or profile)
    pub kind: String,

    /// Name to remove
    pub name: String,
}

/// Execute registry list tool
pub async fn list_tool(args: RegistryListArgs) -> Result<CallToolResult, McpError> {
    // Load fresh config from disk
    let config = crate::config::load_config()
        .map_err(|e| McpError::internal_error(format!("Failed to load config: {}", e), None))?;

    let mut output = String::new();

    match args.target.as_str() {
        "components" | "all" => {
            output.push_str("## Components\n\n");
            if config.components.is_empty() {
                output.push_str("No components registered.\n");
            } else {
                let mut components: Vec<_> = config.components.iter().collect();
                components.sort_by_key(|(name, _)| *name);
                for (name, spec) in components {
                    output.push_str(&format!("- `{}` → {}\n", name, spec));
                }
            }
            output.push('\n');
        }
        _ => {}
    }

    match args.target.as_str() {
        "profiles" | "all" => {
            output.push_str("## Profiles\n\n");
            if config.profiles.is_empty() {
                output.push_str("No profiles registered.\n");
            } else {
                let mut profile_names: Vec<_> = config.profiles.keys().collect();
                profile_names.sort();
                for name in profile_names {
                    let profile = &config.profiles[name];
                    output.push_str(&format!("### `{}`\n", name));
                    if let Some(base) = &profile.base {
                        output.push_str(&format!("  Base: {}\n", base));
                    }
                    output.push_str(&format!(
                        "  Components: {}\n",
                        profile.components.join(", ")
                    ));
                    output.push_str(&format!("  Output: {}\n\n", profile.output));
                }
            }
        }
        _ => {}
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Execute registry add component tool
pub async fn add_component_tool(args: AddComponentArgs) -> Result<CallToolResult, McpError> {
    let output = Command::new("wasmcp")
        .args(["registry", "component", "add", &args.alias, &args.spec])
        .output()
        .await
        .map_err(|e| {
            McpError::internal_error(
                format!("Failed to execute registry component add: {}", e),
                Some(serde_json::json!({
                    "error": e.to_string(),
                    "command": "wasmcp registry component add",
                    "alias": args.alias,
                    "spec": args.spec,
                })),
            )
        })?;

    if output.status.success() {
        Ok(CallToolResult::success(vec![Content::text(format!(
            "✓ Added component alias: {} → {}",
            args.alias, args.spec
        ))]))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(McpError::internal_error(
            format!("Failed to add component: {}", stderr),
            Some(serde_json::json!({
                "exit_code": output.status.code(),
                "stderr": stderr.to_string(),
                "alias": args.alias,
                "spec": args.spec,
            })),
        ))
    }
}

/// Execute registry add profile tool
pub async fn add_profile_tool(args: AddProfileArgs) -> Result<CallToolResult, McpError> {
    let mut cmd = Command::new("wasmcp");
    cmd.args(["registry", "profile", "add", &args.name]);

    for component in &args.components {
        cmd.arg(component);
    }

    if let Some(output_path) = &args.output {
        cmd.arg("--output").arg(output_path);
    } else {
        // Default output required
        return Err(McpError::invalid_params(
            "Output path is required for profile creation",
            None,
        ));
    }

    let output = cmd.output().await.map_err(|e| {
        McpError::internal_error(
            format!("Failed to execute registry profile add: {}", e),
            Some(serde_json::json!({
                "error": e.to_string(),
                "command": "wasmcp registry profile add",
                "profile_name": args.name,
                "components": args.components,
            })),
        )
    })?;

    if output.status.success() {
        Ok(CallToolResult::success(vec![Content::text(format!(
            "✓ Added profile: {}",
            args.name
        ))]))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(McpError::internal_error(
            format!("Failed to add profile: {}", stderr),
            Some(serde_json::json!({
                "exit_code": output.status.code(),
                "stderr": stderr.to_string(),
                "profile_name": args.name,
                "components": args.components,
                "output": args.output,
            })),
        ))
    }
}

/// Execute registry remove tool
pub async fn remove_tool(args: RemoveArgs) -> Result<CallToolResult, McpError> {
    let output = Command::new("wasmcp")
        .args(["registry", &args.kind, "remove", &args.name])
        .output()
        .await
        .map_err(|e| {
            McpError::internal_error(
                format!("Failed to execute registry remove: {}", e),
                Some(serde_json::json!({
                    "error": e.to_string(),
                    "command": "wasmcp registry remove",
                    "kind": args.kind,
                    "name": args.name,
                })),
            )
        })?;

    if output.status.success() {
        Ok(CallToolResult::success(vec![Content::text(format!(
            "✓ Removed {} '{}'",
            args.kind, args.name
        ))]))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(McpError::internal_error(
            format!("Failed to remove: {}", stderr),
            Some(serde_json::json!({
                "exit_code": output.status.code(),
                "stderr": stderr.to_string(),
                "kind": args.kind,
                "name": args.name,
            })),
        ))
    }
}
