use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

use crate::util::find_repo_root;

use super::constants::{get_workflow_name, COMPONENTS, MCP_WIT_NAMESPACE};
use super::display::{display_component_versions, display_version_match_status};
use super::file_ops::{
    get_cargo_version, get_local_wit_dependencies, get_wit_version, update_cargo_version,
    update_deps_toml_mcp_version, update_wit_dependency_versions, update_wit_package_version,
};
use super::github::{get_latest_component_release, get_latest_mcp_wit_release};
use super::version::Version;

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
            Some(release) => self.local_version > *release,
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
    let latest_release = get_latest_component_release(component).unwrap_or(None);
    let local_mcp_wit = get_local_wit_dependencies(component, repo_root).unwrap_or(None);

    display_component_versions(
        component,
        &cargo_version,
        wit_version.as_ref(),
        latest_release.as_ref(),
        local_mcp_wit.as_deref(),
    )?;

    display_version_match_status(&cargo_version, wit_version.as_ref());

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
        let latest_release = get_latest_component_release(component).unwrap_or(None);
        let local_mcp_wit = get_local_wit_dependencies(component, repo_root).unwrap_or(None);

        println!("\x1b[34mChecking {}...\x1b[0m", component);

        display_component_versions(
            component,
            &cargo_version,
            wit_version.as_ref(),
            latest_release.as_ref(),
            local_mcp_wit.as_deref(),
        )?;

        if let Some(wit_ver) = &wit_version {
            if cargo_version != *wit_ver {
                println!("\x1b[31m  ✗ Version mismatch!\x1b[0m");
                println!(
                    "\x1b[31m    Cargo.toml has {} but world.wit has {}\x1b[0m",
                    cargo_version, wit_ver
                );
                failed = true;
            } else {
                println!("\x1b[32m  ✓ Versions match\x1b[0m");
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

        let wit_version = get_wit_version(component, &repo_root).unwrap_or(None);
        let latest_release = get_latest_component_release(component).unwrap_or(None);

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
            current_version.bump_patch()
        } else if bump_count > 1 {
            bail!("Can only specify one of --patch, --minor, or --major");
        } else if major {
            current_version.bump_major()
        } else if minor {
            current_version.bump_minor()
        } else {
            current_version.bump_patch()
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

    if new_version <= current_version {
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
        println!(
            "\x1b[33m[DRY RUN] Would update {} from {} to {}\x1b[0m",
            component, current_version, new_version
        );
        return Ok(());
    }

    if !force {
        println!("\x1b[34m==================================================================\x1b[0m");
        println!(
            "\x1b[33mBump {} from {} to {}?\x1b[0m",
            component, current_version, new_version
        );
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
    update_cargo_version(&cargo_toml_path, &new_version)?;
    println!("\x1b[32m✓ Updated Cargo.toml\x1b[0m");

    // Update world.wit package version
    if world_wit_path.exists() {
        update_wit_package_version(&world_wit_path, &new_version)?;
        println!("\x1b[32m✓ Updated world.wit package version\x1b[0m");
    }

    println!();
    println!(
        "\x1b[32m✓ Successfully bumped {} from {} to {}\x1b[0m",
        component, current_version, new_version
    );

    Ok(())
}

pub fn update_deps(
    component: Option<String>,
    version: Option<String>,
    latest: bool,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    let repo_root = find_repo_root()?;

    // Determine target MCP WIT version
    let target_version = if let Some(explicit_version) = version {
        explicit_version
    } else if latest {
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

    if !dry_run && !force {
        println!("\x1b[33mUpdate {} component(s)?\x1b[0m", components_to_update.len());
        print!("Enter 'yes' to proceed: ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim() != "yes" {
            println!("\x1b[33mUpdate cancelled\x1b[0m");
            return Ok(());
        }
        println!();
    }

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
        let current_mcp_wit =
            get_local_wit_dependencies(comp, &repo_root)?.unwrap_or_else(|| "(none)".to_string());

        if current_mcp_wit == target_version {
            println!("\x1b[32m✓ {} already at {}\x1b[0m", comp, target_version);
            continue;
        }

        println!(
            "\x1b[33m→ {}: {} → {}\x1b[0m",
            comp, current_mcp_wit, target_version
        );

        if dry_run {
            continue;
        }

        // Update world.wit exports/imports
        update_wit_dependency_versions(&world_wit_path, MCP_WIT_NAMESPACE, &target_version)?;

        // Update deps.toml
        if deps_toml_path.exists() {
            update_deps_toml_mcp_version(&deps_toml_path, &target_version)?;
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
        println!(
            "\x1b[33m[DRY RUN] Would update MCP WIT dependencies to {}\x1b[0m",
            target_version
        );
    } else {
        println!(
            "\x1b[32m✓ Successfully updated MCP WIT dependencies to {}\x1b[0m",
            target_version
        );
    }

    Ok(())
}

fn show_detailed_component_info(component: &str, repo_root: &Path) -> Result<()> {
    let cargo_version = get_cargo_version(component, repo_root)?;
    let wit_version = get_wit_version(component, repo_root)?;
    let latest_release = get_latest_component_release(component)?;

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

        if cargo_version == *release {
            println!("\x1b[33m  ⚠ Warning: Local version equals released version\x1b[0m");
            println!("    This may fail if the release already exists");
        } else if cargo_version > *release {
            println!("\x1b[32m  ✓ Local version is newer\x1b[0m");
        } else {
            println!("\x1b[31m  ✗ Local version is OLDER than released version\x1b[0m");
            println!("    Cannot release an older version");
        }
    } else {
        println!("\x1b[33m  Latest:      (none - this will be the first release)\x1b[0m");
    }
    println!();

    println!("\x1b[33mWorkflow Details:\x1b[0m");
    println!("  Workflow:    {}", get_workflow_name(component));
    println!("  Tag:         {}-v{}", component, cargo_version);
    println!("  Package:     wasmcp:{}@{}", component, cargo_version);
    println!(
        "  Registry:    ghcr.io/wasmcp/{}:{}",
        component, cargo_version
    );
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

pub fn publish_releases(
    components: Vec<String>,
    dry_run: bool,
    force: bool,
    verbose: bool,
) -> Result<()> {
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

            let latest_release = get_latest_component_release(component).unwrap_or(None);

            let needs = match &latest_release {
                None => true,
                Some(release) => cargo_version > *release,
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
            println!(
                "\x1b[33m[DRY RUN] Would trigger {} release workflow(s)\x1b[0m",
                to_publish.len()
            );
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
            println!(
                "\x1b[33m[DRY RUN] Would trigger {} release workflow(s)\x1b[0m",
                to_publish.len()
            );
            return Ok(());
        }
    }

    if !force {
        println!("\x1b[34m==================================================================\x1b[0m");
        println!(
            "\x1b[33mTrigger {} release workflow(s)?\x1b[0m",
            to_publish.len()
        );
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

        println!(
            "\x1b[34mPublishing: {} - v{} from {}\x1b[0m",
            component, cargo_version, branch
        );

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
            println!(
                "\x1b[32m✓ Triggered {} v{}\x1b[0m",
                component, cargo_version
            );
            succeeded.push(component.clone());
        } else {
            println!(
                "\x1b[31m✗ Failed to trigger {} v{}\x1b[0m",
                component, cargo_version
            );
            failed.push(component.clone());
        }
    }

    println!();
    println!("\x1b[34m==================================================================\x1b[0m");

    if !succeeded.is_empty() {
        println!(
            "\x1b[32m✓ Successfully triggered {} workflow(s)\x1b[0m",
            succeeded.len()
        );
    }

    if !failed.is_empty() {
        println!(
            "\x1b[31m✗ Failed to trigger {} workflow(s)\x1b[0m",
            failed.len()
        );
        for component in &failed {
            println!("  - {}", component);
        }
    }

    println!();
    println!("Monitor workflows at:");

    let output = Command::new("gh")
        .args(&[
            "repo",
            "view",
            "--json",
            "nameWithOwner",
            "-q",
            ".nameWithOwner",
        ])
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
        bail!(
            "versions.toml not found at {}",
            versions_toml_path.display()
        );
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
                        updates.push((
                            "mcp-v20250618".to_string(),
                            current_version,
                            latest_version,
                        ));
                    }
                }
                break;
            }
        }
    }

    // Check each component for latest version
    for component in COMPONENTS {
        let latest_release = get_latest_component_release(component)?;

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
        println!(
            "\x1b[33m[DRY RUN] Would update {} version(s) in versions.toml\x1b[0m",
            updates.len()
        );
        return Ok(());
    }

    if !force {
        println!("\x1b[34m==================================================================\x1b[0m");
        println!(
            "\x1b[33mUpdate {} version(s) in versions.toml?\x1b[0m",
            updates.len()
        );
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

    println!(
        "\x1b[32m✓ Successfully updated {} version(s) in versions.toml\x1b[0m",
        updates.len()
    );

    Ok(())
}
