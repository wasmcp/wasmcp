//! Composition tool
//!
//! Provides MCP tool interface for composing WASM components into MCP servers.

use rmcp::ErrorData as McpError;
use rmcp::model::*;
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::process::Command;

/// Tool parameter types for compose
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

/// Execute the compose tool
pub async fn execute(args: ComposeArgs) -> Result<CallToolResult, McpError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test ComposeArgs deserialization with all fields
    #[test]
    fn test_compose_args_all_fields() {
        let json = serde_json::json!({
            "components": ["comp1.wasm", "comp2.wasm"],
            "output": "server.wasm",
            "transport": "http",
            "force": true,
            "verbose": true,
            "version": "0.1.0",
            "deps_dir": "/tmp/deps",
            "skip_download": true,
            "override_transport": "custom-transport.wasm",
            "override_method_not_found": "custom-mnf.wasm"
        });

        let args: ComposeArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.components, vec!["comp1.wasm", "comp2.wasm"]);
        assert_eq!(args.output, Some("server.wasm".to_string()));
        assert_eq!(args.transport, Some("http".to_string()));
        assert_eq!(args.force, Some(true));
        assert_eq!(args.verbose, Some(true));
        assert_eq!(args.version, Some("0.1.0".to_string()));
        assert_eq!(args.deps_dir, Some("/tmp/deps".to_string()));
        assert_eq!(args.skip_download, Some(true));
        assert_eq!(
            args.override_transport,
            Some("custom-transport.wasm".to_string())
        );
        assert_eq!(
            args.override_method_not_found,
            Some("custom-mnf.wasm".to_string())
        );
    }

    /// Test ComposeArgs with minimal required fields
    #[test]
    fn test_compose_args_minimal() {
        let json = serde_json::json!({
            "components": ["component.wasm"]
        });

        let args: ComposeArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.components, vec!["component.wasm"]);
        assert_eq!(args.output, None);
        assert_eq!(args.transport, None);
        assert_eq!(args.force, None);
        assert_eq!(args.verbose, None);
        assert_eq!(args.version, None);
        assert_eq!(args.deps_dir, None);
        assert_eq!(args.skip_download, None);
    }

    /// Test ComposeArgs with empty components list
    #[test]
    fn test_compose_args_empty_components() {
        let json = serde_json::json!({
            "components": []
        });

        let args: ComposeArgs = serde_json::from_value(json).unwrap();
        assert!(args.components.is_empty());
    }

    /// Test ComposeArgs missing required field fails
    #[test]
    fn test_compose_args_missing_components() {
        let json = serde_json::json!({
            "output": "server.wasm"
        });

        let result: Result<ComposeArgs, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    /// Test transport validation values
    #[test]
    fn test_transport_values() {
        let http = serde_json::json!({
            "components": ["test.wasm"],
            "transport": "http"
        });
        let args: ComposeArgs = serde_json::from_value(http).unwrap();
        assert_eq!(args.transport, Some("http".to_string()));

        let stdio = serde_json::json!({
            "components": ["test.wasm"],
            "transport": "stdio"
        });
        let args: ComposeArgs = serde_json::from_value(stdio).unwrap();
        assert_eq!(args.transport, Some("stdio".to_string()));
    }

    /// Test boolean flags default to None when not specified
    #[test]
    fn test_boolean_defaults() {
        let json = serde_json::json!({
            "components": ["test.wasm"]
        });

        let args: ComposeArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.force, None);
        assert_eq!(args.verbose, None);
        assert_eq!(args.skip_download, None);
    }

    /// Test boolean flags can be false
    #[test]
    fn test_boolean_false_values() {
        let json = serde_json::json!({
            "components": ["test.wasm"],
            "force": false,
            "verbose": false
        });

        let args: ComposeArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.force, Some(false));
        assert_eq!(args.verbose, Some(false));
    }

    /// Test override fields
    #[test]
    fn test_override_fields() {
        let json = serde_json::json!({
            "components": ["test.wasm"],
            "override_transport": "my-transport.wasm",
            "override_method_not_found": "my-mnf.wasm"
        });

        let args: ComposeArgs = serde_json::from_value(json).unwrap();
        assert_eq!(
            args.override_transport,
            Some("my-transport.wasm".to_string())
        );
        assert_eq!(
            args.override_method_not_found,
            Some("my-mnf.wasm".to_string())
        );
    }

    /// Test multiple components
    #[test]
    fn test_multiple_components() {
        let json = serde_json::json!({
            "components": [
                "calculator.wasm",
                "weather.wasm",
                "database.wasm"
            ]
        });

        let args: ComposeArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.components.len(), 3);
        assert_eq!(args.components[0], "calculator.wasm");
        assert_eq!(args.components[1], "weather.wasm");
        assert_eq!(args.components[2], "database.wasm");
    }

    /// Test version string format
    #[test]
    fn test_version_formats() {
        let semver = serde_json::json!({
            "components": ["test.wasm"],
            "version": "1.2.3"
        });
        let args: ComposeArgs = serde_json::from_value(semver).unwrap();
        assert_eq!(args.version, Some("1.2.3".to_string()));

        let with_v = serde_json::json!({
            "components": ["test.wasm"],
            "version": "v0.4.3"
        });
        let args: ComposeArgs = serde_json::from_value(with_v).unwrap();
        assert_eq!(args.version, Some("v0.4.3".to_string()));
    }
}
