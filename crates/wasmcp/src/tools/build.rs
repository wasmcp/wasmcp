use anyhow::Result;
use rmcp::model::{CallToolResult, Content};
use serde_json::Value;
use std::process::Command;
use std::path::Path;

/// Build and compose provider with transport
/// This is language-agnostic - all projects must have a Makefile
pub async fn wasmcp_build(args: Option<Value>) -> Result<CallToolResult> {
    let path = args
        .as_ref()
        .and_then(|a| a.get("path"))
        .and_then(|p| p.as_str())
        .unwrap_or(".");

    let project_path = Path::new(path);
    
    // Check for Makefile
    let makefile = project_path.join("Makefile");
    if !makefile.exists() {
        return Ok(CallToolResult::error(vec![Content::text(
            "❌ No Makefile found in project directory.\n\n\
            All wasmcp projects must have a Makefile that handles:\n\
            - Building the component (language-specific)\n\
            - Composing with transport (using wac)\n\n\
            Example Makefile targets:\n\
            - make build: Build and compose the component\n\
            - make clean: Clean build artifacts"
        )]));
    }
    
    // Run make build
    let output = Command::new("make")
        .arg("build")
        .current_dir(project_path)
        .output()?;
    
    if output.status.success() {
        // Check if the composed output exists
        let composed_wasm = project_path.join("mcp-http-server.wasm");
        if composed_wasm.exists() {
            Ok(CallToolResult::success(vec![Content::text(
                "✅ Build successful! Component built and composed to mcp-http-server.wasm\n\n\
                You can now:\n\
                - Run with Spin: wasmcp_serve_spin\n\
                - Run with Wasmtime: wasmcp_serve_wasmtime"
            )]))
        } else {
            Ok(CallToolResult::success(vec![Content::text(
                "✅ Build completed successfully!\n\n\
                Note: mcp-http-server.wasm not found after build.\n\
                Check your Makefile's build target to ensure it composes the component."
            )]))
        }
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        Ok(CallToolResult::error(vec![Content::text(format!(
            "❌ Build failed!\n\n\
            Command: make build\n\
            Directory: {}\n\n\
            Error output:\n{}\n\n\
            Standard output:\n{}",
            project_path.display(),
            error,
            stdout
        ))]))
    }
}