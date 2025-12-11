use anyhow::{anyhow, bail, Context, Result};
use std::process::Command;

use super::constants::{GH_RELEASE_FETCH_LIMIT, MCP_WIT_TAG_PREFIX, MCP_WIT_NAMESPACE};
use super::version::Version;

/// Get the latest release matching a version prefix pattern
fn get_latest_release_with_prefix(prefix: &str) -> Result<Option<Version>> {
    let output = Command::new("gh")
        .args(&[
            "release",
            "list",
            "--exclude-drafts",
            "--exclude-pre-releases",
            "--limit",
            GH_RELEASE_FETCH_LIMIT,
        ])
        .output()
        .context("Failed to run gh release list")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh release list failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut versions = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        for part in &parts {
            if let Some(version_str) = part.strip_prefix(prefix) {
                if let Ok(version) = Version::parse(version_str) {
                    versions.push(version);
                    break;
                }
            }
        }
    }

    if versions.is_empty() {
        Ok(None)
    } else {
        versions.sort_by(|a, b| b.cmp(a)); // Sort descending
        Ok(versions.into_iter().next())
    }
}

pub fn get_latest_component_release(component: &str) -> Result<Option<Version>> {
    let prefix = format!("{}-v", component);
    get_latest_release_with_prefix(&prefix)
}

pub fn get_latest_mcp_wit_release() -> Result<Option<Version>> {
    get_latest_release_with_prefix(MCP_WIT_TAG_PREFIX)
}

pub fn get_published_component_wit_deps(
    component: &str,
    version: &Version,
) -> Result<Option<String>> {
    let package_name = format!("wasmcp:{}@{}", component, version);
    let temp_file = std::env::temp_dir().join(format!("{}.wasm", component));

    let temp_file_str = temp_file
        .to_str()
        .ok_or_else(|| anyhow!("Temp file path contains invalid UTF-8"))?;

    // Download using wkg
    let output = Command::new("wkg")
        .args(&["get", &package_name, "--output", temp_file_str, "--overwrite"])
        .output()
        .context("Failed to run wkg get")?;

    if !output.status.success() {
        // Component not published or download failed
        std::fs::remove_file(&temp_file).ok();
        return Ok(None);
    }

    // Use wasm-tools to inspect the component
    let output = Command::new("wasm-tools")
        .args(&["component", "wit", temp_file_str])
        .output();

    // Clean up temp file before processing results
    std::fs::remove_file(&temp_file).ok();

    let output = output.context("Failed to run wasm-tools component wit")?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Look for MCP WIT namespace import version
    for line in stdout.lines() {
        if line.contains(MCP_WIT_NAMESPACE) && line.contains('@') {
            if let Some(at_pos) = line.rfind('@') {
                let version_part = &line[at_pos + 1..];
                let version_str = version_part
                    .split(|c| c == ';' || c == ',')
                    .next()
                    .unwrap_or("")
                    .trim();
                if !version_str.is_empty() {
                    return Ok(Some(version_str.to_string()));
                }
            }
        }
    }

    Ok(None)
}
