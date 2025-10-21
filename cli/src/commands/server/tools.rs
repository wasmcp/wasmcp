use rmcp::model::*;
use rmcp::ErrorData as McpError;
use schemars::JsonSchema;
use serde::Deserialize;
use std::process::Command;
use crate::config::WasmcpConfig;

// Tool parameter types
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ComposeArgs {
    /// Components to compose (profiles, aliases, or paths)
    pub components: Vec<String>,

    /// Output file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,

    /// Transport type (http or stdio)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RegistryListArgs {
    /// What to list (components, profiles, or all)
    #[serde(default = "default_list_target")]
    pub target: String,
}

fn default_list_target() -> String {
    "all".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddComponentArgs {
    /// Component alias name
    pub alias: String,

    /// Component path or reference
    pub spec: String,
}

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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveArgs {
    /// Type to remove (component or profile)
    pub kind: String,

    /// Name to remove
    pub name: String,
}

// Tool implementations
pub async fn compose_tool(args: ComposeArgs) -> Result<CallToolResult, McpError> {
    let mut cmd = Command::new("wasmcp");
    cmd.arg("compose");

    for component in &args.components {
        cmd.arg(component);
    }

    if let Some(output) = &args.output {
        cmd.arg("-o").arg(output);
    }

    if let Some(transport) = &args.transport {
        cmd.arg("--transport").arg(transport);
    }

    let output = cmd.output()
        .map_err(|e| McpError::internal_error(
            format!("Failed to execute compose: {}", e),
            None
        ))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(CallToolResult::success(vec![
            Content::text(format!("✓ Composition successful\n\n{}", stdout))
        ]))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(McpError::internal_error(
            format!("Composition failed: {}", stderr),
            None
        ))
    }
}

pub async fn registry_list_tool(
    config: &WasmcpConfig,
    args: RegistryListArgs,
) -> Result<CallToolResult, McpError> {
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
            output.push_str("\n");
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
                    output.push_str(&format!("  Components: {}\n", profile.components.join(", ")));
                    output.push_str(&format!("  Output: {}\n\n", profile.output));
                }
            }
        }
        _ => {}
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

pub async fn registry_add_component_tool(
    args: AddComponentArgs,
) -> Result<CallToolResult, McpError> {
    let output = Command::new("wasmcp")
        .args(["registry", "component", "add", &args.alias, &args.spec])
        .output()
        .map_err(|e| McpError::internal_error(
            format!("Failed to add component: {}", e),
            None
        ))?;

    if output.status.success() {
        Ok(CallToolResult::success(vec![
            Content::text(format!("✓ Added component alias: {} → {}", args.alias, args.spec))
        ]))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(McpError::internal_error(
            format!("Failed to add component: {}", stderr),
            None
        ))
    }
}

pub async fn registry_add_profile_tool(
    args: AddProfileArgs,
) -> Result<CallToolResult, McpError> {
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
            None
        ));
    }

    let output = cmd.output()
        .map_err(|e| McpError::internal_error(
            format!("Failed to add profile: {}", e),
            None
        ))?;

    if output.status.success() {
        Ok(CallToolResult::success(vec![
            Content::text(format!("✓ Added profile: {}", args.name))
        ]))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(McpError::internal_error(
            format!("Failed to add profile: {}", stderr),
            None
        ))
    }
}

pub async fn registry_remove_tool(args: RemoveArgs) -> Result<CallToolResult, McpError> {
    let output = Command::new("wasmcp")
        .args(["registry", &args.kind, "remove", &args.name])
        .output()
        .map_err(|e| McpError::internal_error(
            format!("Failed to remove: {}", e),
            None
        ))?;

    if output.status.success() {
        Ok(CallToolResult::success(vec![
            Content::text(format!("✓ Removed {} '{}'", args.kind, args.name))
        ]))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(McpError::internal_error(
            format!("Failed to remove: {}", stderr),
            None
        ))
    }
}
