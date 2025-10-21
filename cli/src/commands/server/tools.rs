use rmcp::ErrorData as McpError;
use rmcp::model::*;
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::process::Command;

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

    /// Overwrite existing output file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,

    /// Enable verbose output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<bool>,

    /// wasmcp version for framework dependencies
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Directory for dependency components
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deps_dir: Option<String>,

    /// Skip downloading dependencies
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_download: Option<bool>,

    /// Override transport component
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_transport: Option<String>,

    /// Override method-not-found component
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_method_not_found: Option<String>,
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

    if args.force == Some(true) {
        cmd.arg("--force");
    }

    if args.verbose == Some(true) {
        cmd.arg("--verbose");
    }

    if let Some(version) = &args.version {
        cmd.arg("--version").arg(version);
    }

    if let Some(deps_dir) = &args.deps_dir {
        cmd.arg("--deps-dir").arg(deps_dir);
    }

    if args.skip_download == Some(true) {
        cmd.arg("--skip-download");
    }

    if let Some(override_transport) = &args.override_transport {
        cmd.arg("--override-transport").arg(override_transport);
    }

    if let Some(override_mnf) = &args.override_method_not_found {
        cmd.arg("--override-method-not-found").arg(override_mnf);
    }

    let output = cmd.output().await.map_err(|e| {
        McpError::internal_error(
            format!("Failed to execute compose: {}", e),
            Some(serde_json::json!({
                "error": e.to_string(),
                "command": "wasmcp compose",
                "components": args.components,
            })),
        )
    })?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(CallToolResult::success(vec![Content::text(format!(
            "✓ Composition successful\n\n{}",
            stdout
        ))]))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(McpError::internal_error(
            format!("Composition failed: {}", stderr),
            Some(serde_json::json!({
                "exit_code": output.status.code(),
                "stderr": stderr.to_string(),
                "stdout": stdout.to_string(),
                "components": args.components,
                "output": args.output,
            })),
        ))
    }
}

pub async fn registry_list_tool(args: RegistryListArgs) -> Result<CallToolResult, McpError> {
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

pub async fn registry_add_component_tool(
    args: AddComponentArgs,
) -> Result<CallToolResult, McpError> {
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

pub async fn registry_add_profile_tool(args: AddProfileArgs) -> Result<CallToolResult, McpError> {
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

pub async fn registry_remove_tool(args: RemoveArgs) -> Result<CallToolResult, McpError> {
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
