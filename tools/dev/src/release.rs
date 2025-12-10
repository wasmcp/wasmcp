use anyhow::{anyhow, bail, Context, Result};
use std::cmp::Ordering;
use std::path::Path;
use std::process::Command;

use crate::util::find_repo_root;

const COMPONENTS: &[&str] = &[
    "authorization",
    "filter-middleware",
    "kv-store",
    "method-not-found",
    "prompts-middleware",
    "resources-middleware",
    "server-io",
    "session-store",
    "tools-middleware",
    "transport",
];

// MCP WIT package namespace to check for version dependencies
const MCP_WIT_NAMESPACE: &str = "wasmcp:mcp-v20250618";

// Mapping for components where workflow name differs from component name
fn get_workflow_name(component: &str) -> String {
    match component {
        "session-store" => "release-sessions.yml".to_string(),
        _ => format!("release-{}.yml", component),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

impl Version {
    fn parse(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            bail!("Invalid version format: {}", s);
        }

        Ok(Version {
            major: parts[0].parse().context("Invalid major version")?,
            minor: parts[1].parse().context("Invalid minor version")?,
            patch: parts[2].parse().context("Invalid patch version")?,
        })
    }

    fn cmp(&self, other: &Version) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => match self.minor.cmp(&other.minor) {
                Ordering::Equal => self.patch.cmp(&other.patch),
                ord => ord,
            },
            ord => ord,
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug)]
struct ComponentInfo {
    name: String,
    local_version: Version,
    wit_version: Option<Version>,
    latest_release: Option<Version>,
}

impl ComponentInfo {
    fn needs_release(&self) -> bool {
        match &self.latest_release {
            None => true,
            Some(release) => self.local_version.cmp(release) == Ordering::Greater,
        }
    }

    fn has_version_mismatch(&self) -> bool {
        if let Some(wit_ver) = &self.wit_version {
            wit_ver != &self.local_version
        } else {
            false
        }
    }

    fn status(&self) -> &str {
        if self.latest_release.is_none() {
            "NEW"
        } else {
            "UPDATE"
        }
    }
}

fn get_cargo_version(component: &str, repo_root: &Path) -> Result<Version> {
    let cargo_toml = repo_root
        .join("crates")
        .join(component)
        .join("Cargo.toml");

    let content = std::fs::read_to_string(&cargo_toml)
        .with_context(|| format!("Failed to read {}", cargo_toml.display()))?;

    for line in content.lines() {
        if line.trim().starts_with("version = \"") {
            let version_str = line
                .split('"')
                .nth(1)
                .ok_or_else(|| anyhow!("Failed to parse version from Cargo.toml"))?;
            return Version::parse(version_str);
        }
    }

    bail!("No version found in {}", cargo_toml.display())
}

fn get_wit_version(component: &str, repo_root: &Path) -> Result<Option<Version>> {
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
            // Extract version from "package wasmcp:authorization@0.1.1;"
            if let Some(at_pos) = line.find('@') {
                let version_part = &line[at_pos + 1..];
                let version_str = version_part
                    .trim_end_matches(';')
                    .trim();
                return Version::parse(version_str).map(Some);
            }
        }
    }

    Ok(None)
}

fn get_local_wit_dependencies(component: &str, repo_root: &Path) -> Result<Option<String>> {
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
            && trimmed.contains("@")
        {
            // Extract version from line like: export wasmcp:mcp-v20250618/server-handler@0.1.8;
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

fn get_latest_release(component: &str) -> Result<Option<Version>> {
    // Use gh release view with the tag pattern to directly get the latest release for this component
    // This is more reliable than listing all releases and filtering
    let _tag_pattern = format!("{}-v*", component);

    // First, try to get the latest release with this tag pattern
    let output = Command::new("gh")
        .args(&[
            "release",
            "list",
            "--exclude-drafts",
            "--exclude-pre-releases",
            "--limit", "1000",  // High limit to ensure we don't miss any
        ])
        .output()
        .context("Failed to run gh release list")?;

    if !output.status.success() {
        bail!("gh release list failed");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Find all releases for this component and get the newest version
    let mut versions = Vec::new();
    let version_prefix = format!("{}-v", component);

    for line in stdout.lines() {
        // Line format: "TITLE    TYPE    TAG_NAME    PUBLISHED_AT"
        // Note: TYPE can be "Latest" or empty, which affects indices
        let parts: Vec<&str> = line.split_whitespace().collect();

        // Find the tag by looking for the version prefix pattern
        for part in &parts {
            if let Some(version_str) = part.strip_prefix(&version_prefix) {
                if let Ok(version) = Version::parse(version_str) {
                    versions.push(version);
                    break; // Found the tag for this line
                }
            }
        }
    }

    // Return the highest version found
    if versions.is_empty() {
        Ok(None)
    } else {
        versions.sort_by(|a, b| b.cmp(a)); // Sort descending
        Ok(Some(versions[0].clone()))
    }
}

fn get_latest_mcp_wit_release() -> Result<Option<Version>> {
    let output = Command::new("gh")
        .args(&[
            "release",
            "list",
            "--exclude-drafts",
            "--exclude-pre-releases",
            "--limit", "1000",
        ])
        .output()
        .context("Failed to run gh release list")?;

    if !output.status.success() {
        bail!("gh release list failed");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Find all MCP WIT releases
    let mut versions = Vec::new();
    let version_prefix = "mcp-v2025-06-18-v";

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();

        // Find the tag by looking for the version prefix pattern
        for part in &parts {
            if let Some(version_str) = part.strip_prefix(version_prefix) {
                if let Ok(version) = Version::parse(version_str) {
                    versions.push(version);
                    break;
                }
            }
        }
    }

    // Return the highest version found
    if versions.is_empty() {
        Ok(None)
    } else {
        versions.sort_by(|a, b| b.cmp(a)); // Sort descending
        Ok(Some(versions[0].clone()))
    }
}

fn get_published_wit_dependencies(component: &str, version: &Version) -> Result<Option<String>> {
    // Download the component using wkg and inspect it with wasm-tools
    let package_name = format!("wasmcp:{}@{}", component, version);

    // Create a temporary file for the download
    let temp_file = std::env::temp_dir().join(format!("{}.wasm", component));

    // Download using wkg
    let output = Command::new("wkg")
        .args(&[
            "get",
            &package_name,
            "--output",
            temp_file.to_str().unwrap(),
            "--overwrite",
        ])
        .output()
        .context("Failed to run wkg get")?;

    if !output.status.success() {
        // Component not published or download failed
        std::fs::remove_file(&temp_file).ok();
        return Ok(None);
    }

    // Use wasm-tools to inspect the component
    let output = Command::new("wasm-tools")
        .args(&["component", "wit", temp_file.to_str().unwrap()])
        .output()
        .context("Failed to run wasm-tools component wit")?;

    // Clean up temp file
    std::fs::remove_file(&temp_file).ok();

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Look for MCP WIT namespace import version
    for line in stdout.lines() {
        if line.contains(MCP_WIT_NAMESPACE) && line.contains("@") {
            // Extract version from import line like: import wasmcp:mcp-v20250618/mcp@0.1.8;
            if let Some(at_pos) = line.rfind('@') {
                let version_part = &line[at_pos + 1..];
                // Take everything until semicolon or comma
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

pub fn check_versions(component: Option<String>) -> Result<()> {
    let repo_root = find_repo_root()?;

    if let Some(comp) = component {
        check_component_version(&comp, &repo_root)?;
    } else {
        check_all_versions(&repo_root)?;
    }

    Ok(())
}

fn check_component_version(component: &str, repo_root: &Path) -> Result<()> {
    println!("\x1b[34mChecking {}...\x1b[0m", component);

    let cargo_version = get_cargo_version(component, repo_root)?;
    let wit_version = get_wit_version(component, repo_root)?;
    let latest_release = get_latest_release(component).ok().flatten();
    let local_mcp_wit = get_local_wit_dependencies(component, repo_root).ok().flatten();

    println!("  Cargo.toml:     {}", cargo_version);
    if let Some(wit_ver) = &wit_version {
        println!("  world.wit:      {}", wit_ver);
    }
    if let Some(local_mcp) = &local_mcp_wit {
        println!("  Local MCP WIT:  {}", local_mcp);
    }

    if let Some(release_ver) = &latest_release {
        println!("  GitHub release: {}", release_ver);

        // Check what MCP WIT version the published component uses
        if let Ok(Some(published_mcp_wit)) = get_published_wit_dependencies(component, release_ver) {
            // Compare with local MCP WIT version
            if let Some(local_mcp) = &local_mcp_wit {
                if published_mcp_wit == *local_mcp {
                    println!("    └─ MCP WIT:   {} \x1b[32m✓\x1b[0m", published_mcp_wit);
                } else {
                    println!(
                        "    └─ MCP WIT:   {} \x1b[33m(local: {})\x1b[0m",
                        published_mcp_wit, local_mcp
                    );
                }
            } else {
                println!("    └─ MCP WIT:   {}", published_mcp_wit);
            }
        }
    } else {
        println!("  GitHub release: (none)");
    }

    if let Some(wit_ver) = &wit_version {
        if cargo_version == *wit_ver {
            println!("\x1b[32m  ✓ Versions match\x1b[0m");
        } else {
            println!("\x1b[31m  ✗ Version mismatch!\x1b[0m");
            println!(
                "\x1b[31m    Cargo.toml has {} but world.wit has {}\x1b[0m",
                cargo_version, wit_ver
            );
            return Ok(());
        }
    }

    println!();
    Ok(())
}

fn check_all_versions(repo_root: &Path) -> Result<()> {
    println!("\x1b[34m==================================================================\x1b[0m");
    println!("\x1b[34mChecking version consistency for all components\x1b[0m");
    println!("\x1b[34m==================================================================\x1b[0m");
    println!();

    let mut failed = false;

    for component in COMPONENTS {
        let cargo_version = get_cargo_version(component, repo_root)?;
        let wit_version = get_wit_version(component, repo_root)?;
        let latest_release = get_latest_release(component).ok().flatten();
        let local_mcp_wit = get_local_wit_dependencies(component, repo_root).ok().flatten();

        println!("\x1b[34mChecking {}...\x1b[0m", component);
        println!("  Cargo.toml:     {}", cargo_version);
        if let Some(wit_ver) = &wit_version {
            println!("  world.wit:      {}", wit_ver);
        }
        if let Some(local_mcp) = &local_mcp_wit {
            println!("  Local MCP WIT:  {}", local_mcp);
        }

        if let Some(release_ver) = &latest_release {
            println!("  GitHub release: {}", release_ver);

            // Check what MCP WIT version the published component uses
            if let Ok(Some(published_mcp_wit)) = get_published_wit_dependencies(component, release_ver) {
                // Compare with local MCP WIT version
                if let Some(local_mcp) = &local_mcp_wit {
                    if published_mcp_wit == *local_mcp {
                        println!("    └─ MCP WIT:   {} \x1b[32m✓\x1b[0m", published_mcp_wit);
                    } else {
                        println!(
                            "    └─ MCP WIT:   {} \x1b[33m(local: {})\x1b[0m",
                            published_mcp_wit, local_mcp
                        );
                    }
                } else {
                    println!("    └─ MCP WIT:   {}", published_mcp_wit);
                }
            }
        } else {
            println!("  GitHub release: (none)");
        }

        if let Some(wit_ver) = &wit_version {
            if cargo_version == *wit_ver {
                println!("\x1b[32m  ✓ Versions match\x1b[0m");
            } else {
                println!("\x1b[31m  ✗ Version mismatch!\x1b[0m");
                println!(
                    "\x1b[31m    Cargo.toml has {} but world.wit has {}\x1b[0m",
                    cargo_version, wit_ver
                );
                failed = true;
            }
        }

        println!();
    }

    println!("\x1b[34m==================================================================\x1b[0m");
    if failed {
        println!("\x1b[31m✗ Some component versions are inconsistent\x1b[0m");
    } else {
        println!("\x1b[32m✓ All component versions are consistent\x1b[0m");
    }

    Ok(())
}

pub fn show_status(verbose: bool) -> Result<()> {
    let repo_root = find_repo_root()?;

    println!("\x1b[34m==================================================================\x1b[0m");
    println!("\x1b[34mRelease Status (Read-Only)\x1b[0m");
    println!("\x1b[34m==================================================================\x1b[0m");
    println!();

    // Collect component information
    let mut components_info: Vec<ComponentInfo> = Vec::new();
    let mut has_errors = false;

    for component in COMPONENTS {
        print!("  Checking {}...", component);
        std::io::Write::flush(&mut std::io::stdout())?;

        let cargo_version = match get_cargo_version(component, &repo_root) {
            Ok(v) => v,
            Err(e) => {
                println!(" \x1b[31m✗ {}\x1b[0m", e);
                has_errors = true;
                continue;
            }
        };

        let wit_version = get_wit_version(component, &repo_root).ok().flatten();
        let latest_release = get_latest_release(component).ok().flatten();

        println!(" done");

        components_info.push(ComponentInfo {
            name: component.to_string(),
            local_version: cargo_version,
            wit_version,
            latest_release,
        });
    }

    println!();

    // Check for version mismatches
    for info in &components_info {
        if info.has_version_mismatch() {
            println!(
                "\x1b[31m✗ {}: Cargo.toml ({}) != world.wit ({})\x1b[0m",
                info.name,
                info.local_version,
                info.wit_version.as_ref().unwrap()
            );
            has_errors = true;
        }
    }

    if has_errors {
        println!();
        println!("\x1b[31m✗ Errors detected - please fix before releasing\x1b[0m");
        bail!("Version consistency errors found");
    }

    // Separate components by status
    let needs_release: Vec<&ComponentInfo> = components_info
        .iter()
        .filter(|c| c.needs_release())
        .collect();

    let up_to_date: Vec<&ComponentInfo> = components_info
        .iter()
        .filter(|c| !c.needs_release())
        .collect();

    // Print overview
    println!("\x1b[34m==================================================================\x1b[0m");
    println!("\x1b[34mComponent Release Status Overview\x1b[0m");
    println!("\x1b[34m==================================================================\x1b[0m");
    println!();

    // Only show up-to-date in verbose mode
    if verbose && !up_to_date.is_empty() {
        println!(
            "\x1b[32m✓ Up to Date ({} components):\x1b[0m",
            up_to_date.len()
        );
        println!(
            "  {:<25} {:<15} {:<15}",
            "Component", "Local Version", "Released"
        );
        println!(
            "  {:<25} {:<15} {:<15}",
            "-------------------------", "---------------", "---------------"
        );
        for info in &up_to_date {
            println!(
                "  {:<25} {:<15} {:<15}",
                info.name,
                info.local_version,
                info.latest_release
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_default()
            );
        }
        println!();
    } else if !verbose && !up_to_date.is_empty() {
        println!(
            "\x1b[32m✓ {} components up to date\x1b[0m (use --verbose to show)",
            up_to_date.len()
        );
        println!();
    }

    if !needs_release.is_empty() {
        println!(
            "\x1b[33m→ Needs Release ({} components):\x1b[0m",
            needs_release.len()
        );
        println!(
            "  {:<25} {:<15} {:<15} {:<10}",
            "Component", "Local Version", "Released", "Status"
        );
        println!(
            "  {:<25} {:<15} {:<15} {:<10}",
            "-------------------------", "---------------", "---------------", "----------"
        );
        for info in &needs_release {
            println!(
                "  {:<25} {:<15} {:<15} {:<10}",
                info.name,
                info.local_version,
                info.latest_release
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or("(none)".to_string()),
                info.status()
            );
        }
        println!();
    }

    println!("\x1b[34m==================================================================\x1b[0m");

    if needs_release.is_empty() {
        println!("\x1b[32m✓ All components are up to date\x1b[0m");
    } else {
        println!();
        println!("\x1b[33mTo publish all needed releases, run:\x1b[0m");
        println!("  dev-tools release publish");
        println!();
        println!("\x1b[33mTo publish specific components:\x1b[0m");
        print!("  dev-tools release publish");
        for info in needs_release.iter().take(3) {
            print!(" {}", info.name);
        }
        if needs_release.len() > 3 {
            print!(" ...");
        }
        println!();
    }

    Ok(())
}

pub fn bump_version(
    component: String,
    patch: bool,
    minor: bool,
    major: bool,
    version: Option<String>,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    let repo_root = find_repo_root()?;

    let crate_dir = repo_root.join("crates").join(&component);
    if !crate_dir.exists() {
        bail!("Component not found: {}", component);
    }

    let current_version = get_cargo_version(&component, &repo_root)?;

    // Determine new version
    let new_version = if let Some(explicit_version) = version {
        Version::parse(&explicit_version)?
    } else {
        // Count how many bump flags are set
        let bump_count = [patch, minor, major].iter().filter(|&&x| x).count();

        if bump_count == 0 {
            // Default to patch
            Version {
                major: current_version.major,
                minor: current_version.minor,
                patch: current_version.patch + 1,
            }
        } else if bump_count > 1 {
            bail!("Can only specify one of --patch, --minor, or --major");
        } else if major {
            Version {
                major: current_version.major + 1,
                minor: 0,
                patch: 0,
            }
        } else if minor {
            Version {
                major: current_version.major,
                minor: current_version.minor + 1,
                patch: 0,
            }
        } else {
            // patch
            Version {
                major: current_version.major,
                minor: current_version.minor,
                patch: current_version.patch + 1,
            }
        }
    };

    println!("\x1b[34m==================================================================\x1b[0m");
    if dry_run {
        println!("\x1b[34mBump Version for {} (DRY RUN)\x1b[0m", component);
    } else {
        println!("\x1b[34mBump Version for {}\x1b[0m", component);
    }
    println!("\x1b[34m==================================================================\x1b[0m");
    println!();
    println!("  Current: {}", current_version);
    println!("  New:     {}", new_version);
    println!();

    if new_version.cmp(&current_version) != Ordering::Greater {
        bail!("New version must be greater than current version");
    }

    let cargo_toml_path = crate_dir.join("Cargo.toml");
    let world_wit_path = crate_dir.join("wit").join("world.wit");

    let files_to_update = vec![
        ("Cargo.toml", cargo_toml_path.clone()),
        ("world.wit", world_wit_path.clone()),
    ];

    println!("\x1b[33mFiles to update:\x1b[0m");
    for (name, path) in &files_to_update {
        println!("  - {}", name);
        if !path.exists() {
            bail!("File not found: {}", path.display());
        }
    }
    println!();

    if dry_run {
        println!("\x1b[33m[DRY RUN] Would update {} from {} to {}\x1b[0m", component, current_version, new_version);
        return Ok(());
    }

    if !force {
        println!("\x1b[34m==================================================================\x1b[0m");
        println!("\x1b[33mBump {} from {} to {}?\x1b[0m", component, current_version, new_version);
        print!("Enter 'yes' to proceed: ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim() != "yes" {
            println!("\x1b[33mBump cancelled\x1b[0m");
            return Ok(());
        }
        println!();
    }

    // Update Cargo.toml
    let cargo_content = std::fs::read_to_string(&cargo_toml_path)?;
    let mut updated_cargo = String::new();
    let mut found_version = false;

    for line in cargo_content.lines() {
        if !found_version && line.trim().starts_with("version = \"") {
            updated_cargo.push_str(&format!("version = \"{}\"\n", new_version));
            found_version = true;
        } else {
            updated_cargo.push_str(line);
            updated_cargo.push('\n');
        }
    }

    if !found_version {
        bail!("Could not find version line in Cargo.toml");
    }

    std::fs::write(&cargo_toml_path, updated_cargo)?;
    println!("\x1b[32m✓ Updated Cargo.toml\x1b[0m");

    // Update world.wit package version
    if world_wit_path.exists() {
        let world_content = std::fs::read_to_string(&world_wit_path)?;
        let mut updated_world = String::new();
        let mut found_package = false;

        for line in world_content.lines() {
            if !found_package && line.trim().starts_with("package ") {
                // Replace version in "package wasmcp:component@X.Y.Z;"
                if let Some(at_pos) = line.find('@') {
                    let before_version = &line[..at_pos];
                    let after_version = if let Some(semi_pos) = line[at_pos..].find(';') {
                        &line[at_pos + semi_pos..]
                    } else {
                        ""
                    };
                    updated_world.push_str(&format!("{}@{}{}\n", before_version, new_version, after_version));
                    found_package = true;
                } else {
                    updated_world.push_str(line);
                    updated_world.push('\n');
                }
            } else {
                updated_world.push_str(line);
                updated_world.push('\n');
            }
        }

        if !found_package {
            bail!("Could not find package line in world.wit");
        }

        std::fs::write(&world_wit_path, updated_world)?;
        println!("\x1b[32m✓ Updated world.wit package version\x1b[0m");
    }

    println!();
    println!("\x1b[32m✓ Successfully bumped {} from {} to {}\x1b[0m", component, current_version, new_version);

    Ok(())
}

pub fn update_deps(
    component: Option<String>,
    version: Option<String>,
    latest: bool,
    dry_run: bool,
    _force: bool,
) -> Result<()> {
    let repo_root = find_repo_root()?;

    // Determine target MCP WIT version
    let target_version = if let Some(explicit_version) = version {
        explicit_version
    } else if latest {
        // TODO: Query registry for latest MCP WIT version
        bail!("--latest not yet implemented (specify --version explicitly)");
    } else {
        bail!("Must specify --version or --latest");
    };

    let components_to_update = if let Some(comp) = component {
        vec![comp]
    } else {
        COMPONENTS.iter().map(|&s| s.to_string()).collect()
    };

    println!("\x1b[34m==================================================================\x1b[0m");
    if dry_run {
        println!("\x1b[34mUpdate MCP WIT Dependencies (DRY RUN)\x1b[0m");
    } else {
        println!("\x1b[34mUpdate MCP WIT Dependencies\x1b[0m");
    }
    println!("\x1b[34m==================================================================\x1b[0m");
    println!();
    println!("  Target MCP WIT version: {}", target_version);
    println!("  Components to update:   {}", components_to_update.len());
    println!();

    for comp in &components_to_update {
        let crate_dir = repo_root.join("crates").join(comp);
        if !crate_dir.exists() {
            println!("\x1b[33m⚠ Skipping {} (not found)\x1b[0m", comp);
            continue;
        }

        let world_wit_path = crate_dir.join("wit").join("world.wit");
        let deps_toml_path = crate_dir.join("wit").join("deps.toml");

        if !world_wit_path.exists() {
            println!("\x1b[33m⚠ Skipping {} (no world.wit)\x1b[0m", comp);
            continue;
        }

        // Check current MCP WIT version
        let current_mcp_wit = get_local_wit_dependencies(comp, &repo_root)?.unwrap_or_else(|| "(none)".to_string());

        if current_mcp_wit == target_version {
            println!("\x1b[32m✓ {} already at {}\x1b[0m", comp, target_version);
            continue;
        }

        println!("\x1b[33m→ {}: {} → {}\x1b[0m", comp, current_mcp_wit, target_version);

        if dry_run {
            continue;
        }

        // Update world.wit exports/imports
        let world_content = std::fs::read_to_string(&world_wit_path)?;
        let mut updated_world = String::new();

        for line in world_content.lines() {
            let trimmed = line.trim();
            if (trimmed.starts_with("import ") || trimmed.starts_with("export "))
                && trimmed.contains(MCP_WIT_NAMESPACE)
                && trimmed.contains("@")
            {
                // Replace version in line like: export wasmcp:mcp-v20250618/server-handler@0.1.8;
                if let Some(at_pos) = trimmed.rfind('@') {
                    let before_version = &trimmed[..at_pos];
                    let after_version = if let Some(semi_pos) = trimmed[at_pos..].find(';') {
                        &trimmed[at_pos + semi_pos..]
                    } else {
                        ""
                    };

                    // Preserve indentation
                    let indent = &line[..line.len() - trimmed.len()];
                    updated_world.push_str(&format!("{}{}@{}{}\n", indent, before_version, target_version, after_version));
                } else {
                    updated_world.push_str(line);
                    updated_world.push('\n');
                }
            } else {
                updated_world.push_str(line);
                updated_world.push('\n');
            }
        }

        std::fs::write(&world_wit_path, updated_world)?;

        // Update deps.toml
        if deps_toml_path.exists() {
            let deps_content = std::fs::read_to_string(&deps_toml_path)?;
            let mut updated_deps = String::new();

            for line in deps_content.lines() {
                if line.contains("mcp-v20250618") && line.contains("=") {
                    // Update URL to target version
                    // Format: mcp-v20250618 = "https://github.com/wasmcp/wasmcp/releases/download/mcp-v2025-06-18-vX.Y.Z/wasmcp-mcp-v2025-06-18-X.Y.Z-source.tar.gz"
                    let new_url = format!(
                        "mcp-v20250618 = \"https://github.com/wasmcp/wasmcp/releases/download/mcp-v2025-06-18-v{}/wasmcp-mcp-v2025-06-18-{}-source.tar.gz\"",
                        target_version, target_version
                    );
                    updated_deps.push_str(&new_url);
                    updated_deps.push('\n');
                } else {
                    updated_deps.push_str(line);
                    updated_deps.push('\n');
                }
            }

            std::fs::write(&deps_toml_path, updated_deps)?;
        }

        // Run wit-deps update
        let status = Command::new("wit-deps")
            .arg("update")
            .current_dir(&crate_dir)
            .status()
            .context("Failed to run wit-deps update")?;

        if !status.success() {
            bail!("wit-deps update failed for {}", comp);
        }
    }

    println!();
    if dry_run {
        println!("\x1b[33m[DRY RUN] Would update MCP WIT dependencies to {}\x1b[0m", target_version);
    } else {
        println!("\x1b[32m✓ Successfully updated MCP WIT dependencies to {}\x1b[0m", target_version);
    }

    Ok(())
}

fn show_detailed_component_info(component: &str, repo_root: &Path) -> Result<()> {
    let cargo_version = get_cargo_version(component, repo_root)?;
    let wit_version = get_wit_version(component, repo_root)?;
    let latest_release = get_latest_release(component)?;

    println!("\x1b[34m==================================================================\x1b[0m");
    println!("\x1b[34m{}\x1b[0m", component);
    println!("\x1b[34m==================================================================\x1b[0m");
    println!();

    println!("\x1b[33mLocal Information:\x1b[0m");
    println!("  Component:   {}", component);
    println!("  Version:     {}", cargo_version);
    if let Some(wit_ver) = &wit_version {
        println!("  WIT version: {} \x1b[32m✓\x1b[0m", wit_ver);
    }
    println!();

    println!("\x1b[33mGitHub Release Information:\x1b[0m");
    if let Some(release) = &latest_release {
        println!("  Latest:      {}", release);

        match cargo_version.cmp(release) {
            Ordering::Equal => {
                println!("\x1b[33m  ⚠ Warning: Local version equals released version\x1b[0m");
                println!("    This may fail if the release already exists");
            }
            Ordering::Greater => {
                println!("\x1b[32m  ✓ Local version is newer\x1b[0m");
            }
            Ordering::Less => {
                println!("\x1b[31m  ✗ Local version is OLDER than released version\x1b[0m");
                println!("    Cannot release an older version");
            }
        }
    } else {
        println!("\x1b[33m  Latest:      (none - this will be the first release)\x1b[0m");
    }
    println!();

    println!("\x1b[33mWorkflow Details:\x1b[0m");
    println!("  Workflow:    {}", get_workflow_name(component));
    println!("  Tag:         {}-v{}", component, cargo_version);
    println!("  Package:     wasmcp:{}@{}", component, cargo_version);
    println!("  Registry:    ghcr.io/wasmcp/{}:{}", component, cargo_version);
    println!();

    println!("\x1b[33mThe workflow will:\x1b[0m");
    println!("  1. Validate version format");
    println!("  2. Build component from crates/{}", component);
    println!("  3. Publish to ghcr.io/wasmcp registry");
    println!("  4. Generate SBOM (Software Bill of Materials)");
    println!("  5. Create GitHub release with artifacts");
    println!("  6. Tag release as {}-v{}", component, cargo_version);
    println!();

    Ok(())
}

pub fn publish_releases(components: Vec<String>, dry_run: bool, force: bool, verbose: bool) -> Result<()> {
    let repo_root = find_repo_root()?;

    // Determine which components to publish
    let to_publish = if components.is_empty() {
        // Auto-detect components that need releases
        let mut needs_release = Vec::new();

        for component in COMPONENTS {
            let cargo_version = get_cargo_version(component, &repo_root)?;
            let wit_version = get_wit_version(component, &repo_root)?;

            // Check version consistency
            if let Some(wit_ver) = &wit_version {
                if cargo_version != *wit_ver {
                    println!(
                        "\x1b[31m✗ Skipping {}: version mismatch (Cargo.toml: {}, world.wit: {})\x1b[0m",
                        component, cargo_version, wit_ver
                    );
                    continue;
                }
            }

            let latest_release = get_latest_release(component).ok().flatten();

            let needs = match &latest_release {
                None => true,
                Some(release) => cargo_version.cmp(release) == Ordering::Greater,
            };

            if needs {
                needs_release.push(component.to_string());
            }
        }

        if needs_release.is_empty() {
            println!("\x1b[32m✓ All components are up to date\x1b[0m");
            return Ok(());
        }

        needs_release
    } else {
        components
    };

    // Show detailed info in verbose mode
    if verbose {
        for component in &to_publish {
            show_detailed_component_info(component, &repo_root)?;
        }

        if dry_run {
            println!("\x1b[34m==================================================================\x1b[0m");
            println!("\x1b[33m[DRY RUN] Would trigger {} release workflow(s)\x1b[0m", to_publish.len());
            return Ok(());
        }
    } else {
        // Compact summary mode
        println!("\x1b[34m==================================================================\x1b[0m");
        if dry_run {
            println!("\x1b[34mPublish Releases (DRY RUN)\x1b[0m");
        } else {
            println!("\x1b[34mPublish Releases\x1b[0m");
        }
        println!("\x1b[34m==================================================================\x1b[0m");
        println!();
        println!("  Components to publish: {}", to_publish.len());
        println!();

        for component in &to_publish {
            let cargo_version = get_cargo_version(component, &repo_root)?;
            println!("  - {} v{}", component, cargo_version);
        }
        println!();

        if dry_run {
            println!("\x1b[33m[DRY RUN] Would trigger {} release workflow(s)\x1b[0m", to_publish.len());
            return Ok(());
        }
    }

    if !force {
        println!("\x1b[34m==================================================================\x1b[0m");
        println!("\x1b[33mTrigger {} release workflow(s)?\x1b[0m", to_publish.len());
        print!("Enter 'yes' to proceed: ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim() != "yes" {
            println!("\x1b[33mPublish cancelled\x1b[0m");
            return Ok(());
        }
        println!();
    }

    let mut succeeded = Vec::new();
    let mut failed = Vec::new();

    // Get current branch for context
    let branch_output = Command::new("git")
        .args(&["branch", "--show-current"])
        .output()
        .ok();
    let branch = branch_output
        .as_ref()
        .and_then(|o| String::from_utf8(o.stdout.clone()).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    for component in &to_publish {
        let cargo_version = get_cargo_version(component, &repo_root)?;

        println!("\x1b[34mPublishing: {} - v{} from {}\x1b[0m", component, cargo_version, branch);

        let workflow_name = get_workflow_name(component);
        let status = Command::new("gh")
            .args(&[
                "workflow",
                "run",
                &workflow_name,
                "-f",
                &format!("version={}", cargo_version),
            ])
            .status()
            .context("Failed to run gh workflow run")?;

        if status.success() {
            println!("\x1b[32m✓ Triggered {} v{}\x1b[0m", component, cargo_version);
            succeeded.push(component.clone());
        } else {
            println!("\x1b[31m✗ Failed to trigger {} v{}\x1b[0m", component, cargo_version);
            failed.push(component.clone());
        }
    }

    println!();
    println!("\x1b[34m==================================================================\x1b[0m");

    if !succeeded.is_empty() {
        println!("\x1b[32m✓ Successfully triggered {} workflow(s)\x1b[0m", succeeded.len());
    }

    if !failed.is_empty() {
        println!("\x1b[31m✗ Failed to trigger {} workflow(s)\x1b[0m", failed.len());
        for component in &failed {
            println!("  - {}", component);
        }
    }

    println!();
    println!("Monitor workflows at:");

    let output = Command::new("gh")
        .args(&["repo", "view", "--json", "nameWithOwner", "-q", ".nameWithOwner"])
        .output()?;

    if output.status.success() {
        let repo = String::from_utf8_lossy(&output.stdout).trim().to_string();
        println!("  https://github.com/{}/actions", repo);
    }

    if !failed.is_empty() {
        bail!("Some workflows failed to trigger");
    }

    Ok(())
}

pub fn sync_versions(dry_run: bool, force: bool) -> Result<()> {
    let repo_root = find_repo_root()?;
    let versions_toml_path = repo_root.join("cli").join("versions.toml");

    if !versions_toml_path.exists() {
        bail!("versions.toml not found at {}", versions_toml_path.display());
    }

    println!("\x1b[34m==================================================================\x1b[0m");
    if dry_run {
        println!("\x1b[34mSync versions.toml (DRY RUN)\x1b[0m");
    } else {
        println!("\x1b[34mSync versions.toml\x1b[0m");
    }
    println!("\x1b[34m==================================================================\x1b[0m");
    println!();

    // Read current versions.toml
    let content = std::fs::read_to_string(&versions_toml_path)?;
    let mut updates = Vec::new();

    // Check MCP WIT version first
    let mcp_wit_latest = get_latest_mcp_wit_release()?;
    if let Some(latest_version) = mcp_wit_latest {
        let mcp_line = "mcp-v20250618 = ";
        for line in content.lines() {
            if line.trim().starts_with(mcp_line) {
                if let Some(version_str) = line.split('"').nth(1) {
                    let current_version = Version::parse(version_str)?;
                    if current_version != latest_version {
                        updates.push(("mcp-v20250618".to_string(), current_version, latest_version));
                    }
                }
                break;
            }
        }
    }

    // Check each component for latest version
    for component in COMPONENTS {
        let latest_release = get_latest_release(component)?;

        if let Some(latest_version) = latest_release {
            // Find current version in versions.toml
            let component_line = format!("{} = ", component);
            for line in content.lines() {
                if line.trim().starts_with(&component_line) {
                    // Extract current version
                    if let Some(version_str) = line.split('"').nth(1) {
                        let current_version = Version::parse(version_str)?;

                        if current_version != latest_version {
                            updates.push((component.to_string(), current_version, latest_version));
                        }
                    }
                    break;
                }
            }
        }
    }

    if updates.is_empty() {
        println!("\x1b[32m✓ All versions in versions.toml are up to date\x1b[0m");
        return Ok(());
    }

    println!("\x1b[33mUpdates needed:\x1b[0m");
    println!();
    for (component, current, latest) in &updates {
        println!("  {} {} → {}", component, current, latest);
    }
    println!();

    if dry_run {
        println!("\x1b[33m[DRY RUN] Would update {} version(s) in versions.toml\x1b[0m", updates.len());
        return Ok(());
    }

    if !force {
        println!("\x1b[34m==================================================================\x1b[0m");
        println!("\x1b[33mUpdate {} version(s) in versions.toml?\x1b[0m", updates.len());
        print!("Enter 'yes' to proceed: ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim() != "yes" {
            println!("\x1b[33mSync cancelled\x1b[0m");
            return Ok(());
        }
        println!();
    }

    // Update versions.toml
    let mut updated_content = content.clone();
    for (component, _current, latest) in &updates {
        let component_line = format!("{} = ", component);
        let mut new_lines = Vec::new();

        for line in updated_content.lines() {
            if line.trim().starts_with(&component_line) {
                // Replace version while preserving formatting
                let indent = &line[..line.len() - line.trim_start().len()];
                new_lines.push(format!("{}{} = \"{}\"", indent, component, latest));
            } else {
                new_lines.push(line.to_string());
            }
        }

        updated_content = new_lines.join("\n");
        updated_content.push('\n'); // Ensure trailing newline
    }

    std::fs::write(&versions_toml_path, updated_content)?;

    println!("\x1b[32m✓ Successfully updated {} version(s) in versions.toml\x1b[0m", updates.len());

    Ok(())
}
