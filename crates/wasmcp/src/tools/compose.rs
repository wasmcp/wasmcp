use anyhow::{Result, Context};
use rmcp::model::{CallToolResult, Content};
use serde_json::Value;
use std::process::Command;
use std::path::PathBuf;

/// Compose WASM components using wac
pub async fn wasmcp_compose(args: Option<Value>) -> Result<CallToolResult> {
    let provider_path = args
        .as_ref()
        .and_then(|v| v.get("provider"))
        .and_then(|v| v.as_str())
        .unwrap_or("target/wasm32-wasip1/release/provider.wasm");
    
    let transport_path = args
        .as_ref()
        .and_then(|v| v.get("transport"))
        .and_then(|v| v.as_str())
        .unwrap_or("wasmcp:mcp-transport-http@0.1.0");
    
    let output_path = args
        .as_ref()
        .and_then(|v| v.get("output"))
        .and_then(|v| v.as_str())
        .unwrap_or("mcp-http-server.wasm");
    
    let project_path = args
        .as_ref()
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())
        .unwrap_or(".");
    
    // Check if provider component exists
    let provider = PathBuf::from(project_path).join(provider_path);
    if !provider.exists() {
        return Ok(CallToolResult::error(vec![Content::text(
            format!("Error: Provider component not found at {}\nRun cargo component build first.", provider.display())
        )]));
    }
    
    // Run wac compose command
    let output = Command::new("wac")
        .args(&[
            "compose",
            provider.to_str().unwrap(),
            "--plug",
            transport_path,
            "-o",
            output_path,
        ])
        .current_dir(project_path)
        .output()
        .context("Failed to run wac compose")?;
    
    if output.status.success() {
        let output_full_path = PathBuf::from(project_path).join(output_path);
        Ok(CallToolResult::success(vec![Content::text(
            format!(
                "✅ Successfully composed WASM components!\n\n\
                Provider: {}\n\
                Transport: {}\n\
                Output: {}\n\n\
                You can now serve this component with:\n\
                - wasmcp_serve_spin\n\
                - wasmcp_serve_wasmtime\n\
                - wasmcp_serve_wasmtime_serve",
                provider.display(),
                transport_path,
                output_full_path.display()
            )
        )]))
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Ok(CallToolResult::error(vec![Content::text(
            format!("Composition failed:\n{}", error)
        )]))
    }
}

/// Validate WIT definitions
pub async fn wasmcp_validate_wit(args: Option<Value>) -> Result<CallToolResult> {
    let wit_path = args
        .as_ref()
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())
        .unwrap_or("wit");
    
    let project_path = args
        .as_ref()
        .and_then(|v| v.get("project_path"))
        .and_then(|v| v.as_str())
        .unwrap_or(".");
    
    let wit_dir = PathBuf::from(project_path).join(wit_path);
    
    // Check if wit directory exists
    if !wit_dir.exists() {
        return Ok(CallToolResult::error(vec![Content::text(
            format!("Error: WIT directory not found at {}", wit_dir.display())
        )]));
    }
    
    // Run wkg wit check command
    let output = Command::new("wkg")
        .args(&["wit", "check", wit_dir.to_str().unwrap()])
        .current_dir(project_path)
        .output()
        .context("Failed to run wkg wit check")?;
    
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(CallToolResult::success(vec![Content::text(
            format!(
                "✅ WIT definitions are valid!\n\n\
                Directory: {}\n\
                {}\n\n\
                All WIT interface definitions pass validation.",
                wit_dir.display(),
                if stdout.is_empty() { "No issues found." } else { stdout.as_ref() }
            )
        )]))
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Ok(CallToolResult::error(vec![Content::text(
            format!(
                "WIT validation failed:\n{}\n\n\
                Please fix the errors in your WIT definitions.",
                error
            )
        )]))
    }
}