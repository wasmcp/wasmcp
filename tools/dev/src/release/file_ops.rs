use anyhow::{bail, Context, Result};
use std::path::Path;

use super::constants::MCP_WIT_NAMESPACE;
use super::version::Version;

/// Generic line-by-line file updater
pub fn update_lines_in_file<F, R>(path: &Path, mut matcher: F, replacer: R) -> Result<()>
where
    F: FnMut(&str) -> bool,
    R: Fn(&str) -> String,
{
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let updated = content
        .lines()
        .map(|line| {
            if matcher(line) {
                replacer(line)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";

    std::fs::write(path, updated)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    Ok(())
}

pub fn update_cargo_version(cargo_toml: &Path, new_version: &Version) -> Result<()> {
    let mut found_version = false;

    update_lines_in_file(
        cargo_toml,
        |line| {
            if !found_version && line.trim().starts_with("version = \"") {
                found_version = true;
                true
            } else {
                false
            }
        },
        |_| {
            format!("version = \"{}\"", new_version)
        },
    )?;

    if !found_version {
        bail!("Could not find version line in Cargo.toml");
    }

    Ok(())
}

pub fn update_wit_package_version(world_wit: &Path, new_version: &Version) -> Result<()> {
    let mut found_package = false;

    update_lines_in_file(
        world_wit,
        |line| {
            if !found_package && line.trim().starts_with("package ") {
                found_package = true;
                true
            } else {
                false
            }
        },
        |line| {
            if let Some(at_pos) = line.find('@') {
                let before_version = &line[..at_pos];
                let after_version = if let Some(semi_pos) = line[at_pos..].find(';') {
                    &line[at_pos + semi_pos..]
                } else {
                    ""
                };
                format!("{}@{}{}", before_version, new_version, after_version)
            } else {
                line.to_string()
            }
        },
    )?;

    if !found_package {
        bail!("Could not find package line in world.wit");
    }

    Ok(())
}

pub fn update_wit_dependency_versions(
    world_wit: &Path,
    namespace: &str,
    target_version: &str,
) -> Result<()> {
    update_lines_in_file(
        world_wit,
        |line| {
            let trimmed = line.trim();
            (trimmed.starts_with("import ") || trimmed.starts_with("export "))
                && trimmed.contains(namespace)
                && trimmed.contains('@')
        },
        |line| {
            let trimmed = line.trim();
            if let Some(at_pos) = trimmed.rfind('@') {
                let before_version = &trimmed[..at_pos];
                let after_version = if let Some(semi_pos) = trimmed[at_pos..].find(';') {
                    &trimmed[at_pos + semi_pos..]
                } else {
                    ""
                };
                // Preserve indentation
                let indent = &line[..line.len() - trimmed.len()];
                format!(
                    "{}{}@{}{}",
                    indent, before_version, target_version, after_version
                )
            } else {
                line.to_string()
            }
        },
    )?;

    Ok(())
}

pub fn update_deps_toml_mcp_version(deps_toml: &Path, target_version: &str) -> Result<()> {
    update_lines_in_file(
        deps_toml,
        |line| line.contains("mcp-v20250618") && line.contains('='),
        |_| {
            format!(
                "mcp-v20250618 = \"https://github.com/wasmcp/wasmcp/releases/download/mcp-v2025-06-18-v{}/wasmcp-mcp-v2025-06-18-{}-source.tar.gz\"",
                target_version, target_version
            )
        },
    )?;

    Ok(())
}

pub fn get_cargo_version(component: &str, repo_root: &Path) -> Result<Version> {
    let cargo_toml = repo_root.join("crates").join(component).join("Cargo.toml");

    let content = std::fs::read_to_string(&cargo_toml)
        .with_context(|| format!("Failed to read {}", cargo_toml.display()))?;

    for line in content.lines() {
        if line.trim().starts_with("version = \"") {
            let version_str = line
                .split('"')
                .nth(1)
                .ok_or_else(|| anyhow::anyhow!("Failed to parse version from Cargo.toml"))?;
            return Version::parse(version_str);
        }
    }

    bail!("No version found in {}", cargo_toml.display())
}

pub fn get_wit_version(component: &str, repo_root: &Path) -> Result<Option<Version>> {
    let world_wit = repo_root
        .join("crates")
        .join(component)
        .join("wit")
        .join("world.wit");

    if !world_wit.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&world_wit)
        .with_context(|| format!("Failed to read {}", world_wit.display()))?;

    for line in content.lines() {
        if line.trim().starts_with("package ") {
            if let Some(at_pos) = line.find('@') {
                let version_part = &line[at_pos + 1..];
                let version_str = version_part.trim_end_matches(';').trim();
                return Version::parse(version_str).map(Some);
            }
        }
    }

    Ok(None)
}

pub fn get_local_wit_dependencies(component: &str, repo_root: &Path) -> Result<Option<String>> {
    let world_wit = repo_root
        .join("crates")
        .join(component)
        .join("wit")
        .join("world.wit");

    if !world_wit.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&world_wit)
        .with_context(|| format!("Failed to read {}", world_wit.display()))?;

    // Look for MCP WIT namespace import/export version
    for line in content.lines() {
        let trimmed = line.trim();
        if (trimmed.starts_with("import ") || trimmed.starts_with("export "))
            && trimmed.contains(MCP_WIT_NAMESPACE)
            && trimmed.contains('@')
        {
            if let Some(at_pos) = trimmed.rfind('@') {
                let version_part = &trimmed[at_pos + 1..];
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
