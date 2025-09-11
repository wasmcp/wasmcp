use anyhow::Result;
use rmcp::model::{CallToolResult, Content};
use serde_json::Value;
use std::path::Path;
use walkdir::WalkDir;
use std::process::Command;
use std::fs;

/// List all MCP provider projects in the workspace
pub async fn wasmcp_list(args: Option<Value>) -> Result<CallToolResult> {
    let search_path = args
        .as_ref()
        .and_then(|a| a.get("path"))
        .and_then(|p| p.as_str())
        .unwrap_or(".");

    let providers = find_providers(search_path)?;
    
    let mut output = String::from("MCP Providers found:\n\n");
    
    if providers.is_empty() {
        output.push_str("No MCP providers found in the workspace.\n");
    } else {
        for provider in &providers {
            output.push_str(&format!("📦 {}\n", provider.name));
            output.push_str(&format!("   Path: {}\n", provider.path));
            output.push_str(&format!("   Language: {}\n", provider.language));
            if let Some(desc) = &provider.description {
                output.push_str(&format!("   Description: {}\n", desc));
            }
            output.push_str("\n");
        }
    }
    
    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Initialize a new MCP provider project
pub async fn wasmcp_init(args: Option<Value>) -> Result<CallToolResult> {
    let args = args.ok_or_else(|| anyhow::anyhow!("Arguments required"))?;
    
    let language = args.get("language")
        .and_then(|l| l.as_str())
        .unwrap_or("rust");
    
    let name = args.get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| anyhow::anyhow!("Project name is required"))?;
    
    // Check if spin is available
    if which::which("spin").is_ok() {
        // Use spin templates
        let template_name = format!("wasmcp-{}", language);
        
        let output = Command::new("spin")
            .args(&["new", "-t", &template_name, name])
            .output()?;
        
        if output.status.success() {
            let message = format!(
                "✅ Created new {} MCP provider '{}' using Spin template\n\n\
                Next steps:\n\
                1. cd {}\n\
                2. Review the README.md for language-specific setup\n\
                3. Run 'wasmcp build' to build the provider\n\
                4. Run 'wasmcp serve' to test locally",
                language, name, name
            );
            
            Ok(CallToolResult::success(vec![Content::text(message)]))
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Ok(CallToolResult::error(vec![Content::text(format!("Failed to create project: {}", error))]))
        }
    } else {
        // Copy from local templates if they exist
        let template_dir = format!("templates/{}", language);
        
        if Path::new(&template_dir).exists() {
            copy_template(&template_dir, name)?;
            
            let message = format!(
                "✅ Created new {} MCP provider '{}' from local template\n\n\
                Next steps:\n\
                1. cd {}\n\
                2. Review the README.md for language-specific setup\n\
                3. Run 'wasmcp build' to build the provider\n\
                4. Run 'wasmcp serve' to test locally",
                language, name, name
            );
            
            Ok(CallToolResult::success(vec![Content::text(message)]))
        } else {
            let message = format!(
                "❌ Spin is not installed and no local template found for language '{}'\n\n\
                To use Spin templates, install Spin:\n\
                curl -fsSL https://developer.fermyon.com/downloads/install.sh | bash\n\n\
                Or add local templates to the templates/ directory",
                language
            );
            
            Ok(CallToolResult::error(vec![Content::text(message)]))
        }
    }
}

#[derive(Debug)]
struct ProviderInfo {
    name: String,
    path: String,
    language: String,
    description: Option<String>,
}

fn find_providers(search_path: &str) -> Result<Vec<ProviderInfo>> {
    let mut providers = Vec::new();
    
    for entry in WalkDir::new(search_path)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        
        // Look for provider indicators
        if path.is_dir() {
            // Check for spin.toml (Spin-based providers)
            let spin_toml = path.join("spin.toml");
            if spin_toml.exists() {
                if let Ok(content) = fs::read_to_string(&spin_toml) {
                    if content.contains("wasmcp") || content.contains("mcp") {
                        let name = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        
                        let language = detect_language(path);
                        
                        providers.push(ProviderInfo {
                            name: name.clone(),
                            path: path.display().to_string(),
                            language,
                            description: extract_description(&content),
                        });
                    }
                }
            }
            
            // Check for Cargo.toml with wasmcp dependencies
            let cargo_toml = path.join("Cargo.toml");
            if cargo_toml.exists() {
                if let Ok(content) = fs::read_to_string(&cargo_toml) {
                    if content.contains("wasmcp:mcp") {
                        let name = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        
                        providers.push(ProviderInfo {
                            name: name.clone(),
                            path: path.display().to_string(),
                            language: "rust".to_string(),
                            description: extract_cargo_description(&content),
                        });
                    }
                }
            }
        }
    }
    
    Ok(providers)
}

fn detect_language(path: &Path) -> String {
    if path.join("Cargo.toml").exists() {
        "rust".to_string()
    } else if path.join("package.json").exists() {
        if path.join("tsconfig.json").exists() {
            "typescript".to_string()
        } else {
            "javascript".to_string()
        }
    } else if path.join("go.mod").exists() {
        "go".to_string()
    } else if path.join("requirements.txt").exists() || path.join("pyproject.toml").exists() {
        "python".to_string()
    } else {
        "unknown".to_string()
    }
}

fn extract_description(spin_toml: &str) -> Option<String> {
    for line in spin_toml.lines() {
        if line.starts_with("description") {
            return line.split('=')
                .nth(1)
                .map(|s| s.trim().trim_matches('"').to_string());
        }
    }
    None
}

fn extract_cargo_description(cargo_toml: &str) -> Option<String> {
    for line in cargo_toml.lines() {
        if line.starts_with("description") {
            return line.split('=')
                .nth(1)
                .map(|s| s.trim().trim_matches('"').to_string());
        }
    }
    None
}

fn copy_template(template_dir: &str, target_name: &str) -> Result<()> {
    // Simple recursive copy
    fs::create_dir_all(target_name)?;
    
    for entry in WalkDir::new(template_dir) {
        let entry = entry?;
        let relative = entry.path().strip_prefix(template_dir)?;
        let target = Path::new(target_name).join(relative);
        
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    
    Ok(())
}