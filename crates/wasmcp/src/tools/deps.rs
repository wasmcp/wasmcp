use anyhow::Result;
use rmcp::model::{CallToolResult};
use serde_json::{json, Value};
use std::collections::HashMap;

// Go up one level from tools module to access deps module
use super::super::deps;

/// Check and report on external dependencies
pub async fn wasmcp_check_deps(_args: Option<Value>) -> Result<CallToolResult> {
    let (installed, missing) = deps::check_all_dependencies();
    
    // Build list of all tools
    let all_tools = vec![
        "wasmcp_list",
        "wasmcp_init", 
        "wasmcp_build",
        "wasmcp_serve_spin",
        "wasmcp_serve_wasmtime",
        "wasmcp_compose",
        "wasmcp_validate_wit",
        "wasmcp_check_deps"
    ];
    
    // Categorize tools by availability
    let mut available_tools = vec![];
    let mut unavailable_tools = json!({});
    
    for tool in &all_tools {
        if deps::is_tool_available(tool) {
            available_tools.push(*tool);
        } else {
            let missing_deps = deps::get_tool_dependencies(tool);
            if !missing_deps.is_empty() {
                unavailable_tools[tool] = json!(missing_deps);
            }
        }
    }
    
    // Get metadata for missing dependencies using new system
    let mut missing_metadata = HashMap::new();
    for dep_name in &missing {
        if let Some(meta) = deps::get_dependency_metadata(dep_name) {
            missing_metadata.insert(dep_name.clone(), meta);
        }
    }
    
    // Create structured response using MCP's structured content
    let structured_data = json!({
        "dependencies": {
            "installed": installed,
            "missing": missing,
        },
        "tools": {
            "total": all_tools.len(),
            "available_count": available_tools.len(),
            "unavailable_count": all_tools.len() - available_tools.len(),
            "available": available_tools,
            "unavailable": unavailable_tools,
        },
        "missing_dependencies": missing_metadata,  // Structured metadata with guidance
        "summary": {
            "status": if missing.is_empty() { 
                "all_dependencies_installed" 
            } else if available_tools.len() > all_tools.len() / 2 {
                "partial_dependencies"
            } else {
                "missing_critical_dependencies"
            },
            "message": if missing.is_empty() {
                format!("All dependencies are installed. All tools are available.")
            } else {
                format!("{} of {} tools available. Missing dependencies: {}", 
                    available_tools.len(), 
                    all_tools.len(), 
                    missing.join(", "))
            }
        }
    });
    
    // Return structured result - this creates both structured_content 
    // and a text representation in content
    Ok(CallToolResult::structured(structured_data))
}