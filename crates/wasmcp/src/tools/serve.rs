use anyhow::{Result, Context};
use rmcp::model::{CallToolResult, Content};
use serde_json::Value;
use std::process::Command;
use std::path::PathBuf;

/// Serve composed WASM with Spin runtime
pub async fn wasmcp_serve_spin(args: Option<Value>) -> Result<CallToolResult> {
    let path = args
        .as_ref()
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())
        .unwrap_or(".");
    
    let composed_path = args
        .as_ref()
        .and_then(|v| v.get("composed_path"))
        .and_then(|v| v.as_str())
        .unwrap_or("mcp-http-server.wasm");
    
    let port = args
        .as_ref()
        .and_then(|v| v.get("port"))
        .and_then(|v| v.as_u64())
        .unwrap_or(3001) as u16;
    
    // Build path to composed component
    let project_path = PathBuf::from(path);
    let wasm_path = project_path.join(composed_path);
    
    // Check if composed component exists
    if !wasm_path.exists() {
        return Ok(CallToolResult::error(vec![Content::text(
            format!("Error: Composed component not found at {}\nRun wasmcp_build first to create the composed component.", wasm_path.display())
        )]));
    }
    
    // Kill any existing process on the port
    let _ = Command::new("lsof")
        .args(&["-ti", &format!(":{}", port)])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() && !output.stdout.is_empty() {
                let pids = String::from_utf8_lossy(&output.stdout);
                for pid in pids.lines() {
                    let _ = Command::new("kill")
                        .args(&["-9", pid.trim()])
                        .output();
                }
            }
            Some(())
        });
    
    // Start Spin server
    let output = Command::new("spin")
        .args(&[
            "up",
            "--from",
            wasm_path.to_str().unwrap(),
            "--listen",
            &format!("127.0.0.1:{}", port),
        ])
        .current_dir(&project_path)
        .spawn()
        .context("Failed to start Spin server")?;
    
    let server_url = format!("http://127.0.0.1:{}", port);
    
    Ok(CallToolResult::success(vec![Content::text(
        format!(
            "✅ Started Spin server with composed MCP component\n\n\
            Server URL: {}\n\
            Component: {}\n\
            PID: {}\n\n\
            The server is running in the background. To stop it:\n\
            - kill -9 {}\n\
            - Or: lsof -ti:{} | xargs kill -9",
            server_url,
            wasm_path.display(),
            output.id(),
            output.id(),
            port
        )
    )]))
}

/// Serve composed WASM with Wasmtime runtime
pub async fn wasmcp_serve_wasmtime(args: Option<Value>) -> Result<CallToolResult> {
    let path = args
        .as_ref()
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())
        .unwrap_or(".");
    
    let composed_path = args
        .as_ref()
        .and_then(|v| v.get("composed_path"))
        .and_then(|v| v.as_str())
        .unwrap_or("mcp-http-server.wasm");
    
    let port = args
        .as_ref()
        .and_then(|v| v.get("port"))
        .and_then(|v| v.as_u64())
        .unwrap_or(3001) as u16;
    
    // Build path to composed component
    let project_path = PathBuf::from(path);
    let wasm_path = project_path.join(composed_path);
    
    // Check if composed component exists
    if !wasm_path.exists() {
        return Ok(CallToolResult::error(vec![Content::text(
            format!("Error: Composed component not found at {}\nRun wasmcp_build first to create the composed component.", wasm_path.display())
        )]));
    }
    
    // Kill any existing process on the port
    let _ = Command::new("lsof")
        .args(&["-ti", &format!(":{}", port)])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() && !output.stdout.is_empty() {
                let pids = String::from_utf8_lossy(&output.stdout);
                for pid in pids.lines() {
                    let _ = Command::new("kill")
                        .args(&["-9", pid.trim()])
                        .output();
                }
            }
            Some(())
        });
    
    // Start wasmtime serve
    let output = Command::new("wasmtime")
        .args(&[
            "serve",
            "-Scli",
            "--addr",
            &format!("127.0.0.1:{}", port),
            wasm_path.to_str().unwrap(),
        ])
        .current_dir(&project_path)
        .spawn()
        .context("Failed to start Wasmtime server")?;
    
    let server_url = format!("http://127.0.0.1:{}", port);
    
    Ok(CallToolResult::success(vec![Content::text(
        format!(
            "✅ Started Wasmtime server with composed MCP component\n\n\
            Server URL: {}\n\
            Component: {}\n\
            PID: {}\n\n\
            The server is running in the background. To stop it:\n\
            - kill -9 {}\n\
            - Or: lsof -ti:{} | xargs kill -9",
            server_url,
            wasm_path.display(),
            output.id(),
            output.id(),
            port
        )
    )]))
}