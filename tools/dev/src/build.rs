use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Find the workspace root by looking for Cargo.toml with [workspace]
fn find_workspace_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir().context("Failed to get current directory")?;

    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            let content =
                std::fs::read_to_string(&cargo_toml).context("Failed to read Cargo.toml")?;

            // Check if this is a workspace root
            if content.contains("[workspace]") {
                return Ok(current);
            }
        }

        // Try parent directory
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            anyhow::bail!("Could not find workspace root (no Cargo.toml with [workspace])");
        }
    }
}

/// Component build configuration
struct Component {
    name: String,
    path: PathBuf,
    has_makefile: bool,
}

impl Component {
    fn new(name: String, path: PathBuf, has_makefile: bool) -> Self {
        Self {
            name,
            path,
            has_makefile,
        }
    }
}

/// Check if a directory has a Makefile
fn has_makefile(path: &PathBuf) -> bool {
    path.join("Makefile").exists()
}

/// Discover all buildable components from workspace
fn get_components(workspace_root: &PathBuf) -> Result<Vec<Component>> {
    let mut components = Vec::new();

    // Always build CLI first (excluded from workspace but we want it)
    let cli_path = workspace_root.join("cli");
    if cli_path.exists() {
        components.push(Component::new(
            "cli".to_string(),
            cli_path.clone(),
            has_makefile(&cli_path),
        ));
    }

    // Build all workspace members from crates/*
    let crates_dir = workspace_root.join("crates");
    if crates_dir.exists() {
        let mut crate_entries: Vec<_> = std::fs::read_dir(&crates_dir)
            .context("Failed to read crates directory")?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter(|e| {
                let name = e.file_name();
                // Skip excluded crates from workspace
                name != "wit"
                    && name != "http-server-io"
                    && name != "http-transport"
                    && name != "stdio-transport"
            })
            .collect();

        // Sort by name for consistent order
        crate_entries.sort_by_key(|e| e.file_name());

        for entry in crate_entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip dev-tools (don't build ourselves)
            if name == "dev-tools" {
                continue;
            }

            components.push(Component::new(name, path.clone(), has_makefile(&path)));
        }
    }

    // Build examples (excluded from workspace but we want them)
    let examples_dir = workspace_root.join("examples");
    if examples_dir.exists() {
        let mut example_entries: Vec<_> = std::fs::read_dir(&examples_dir)
            .context("Failed to read examples directory")?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();

        // Sort by name for consistent order
        example_entries.sort_by_key(|e| e.file_name());

        for entry in example_entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            components.push(Component::new(name, path.clone(), has_makefile(&path)));
        }
    }

    Ok(components)
}

/// Run wit-deps update for a component
fn wit_deps_update(component: &Component) -> Result<()> {
    let wit_dir = component.path.join("wit");
    if !wit_dir.exists() {
        return Ok(()); // Skip if no wit directory
    }

    // Nuke existing deps directory to ensure clean state
    let deps_dir = wit_dir.join("deps");
    if deps_dir.exists() {
        println!("  Removing old wit/deps...");
        std::fs::remove_dir_all(&deps_dir).context("Failed to remove old deps directory")?;
    }

    println!("  Running wit-deps update...");
    let status = Command::new("wit-deps")
        .arg("update")
        .current_dir(&component.path)
        .status()
        .context("Failed to run wit-deps")?;

    if !status.success() {
        anyhow::bail!("wit-deps update failed for {}", component.name);
    }

    Ok(())
}

/// Build a single component
fn build_component(component: &Component) -> Result<()> {
    println!("\nBuilding: {}", component.name);

    // Update WIT dependencies first
    wit_deps_update(component)?;

    if component.has_makefile {
        // Use make for components with Makefiles
        println!("  Running make work...");
        let status = Command::new("make")
            .arg("work")
            .current_dir(&component.path)
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .status()
            .with_context(|| {
                format!(
                    "Failed to run make in {} (cwd: {:?})",
                    component.name,
                    std::env::current_dir()
                )
            })?;

        if !status.success() {
            anyhow::bail!("make work failed for {}", component.name);
        }
    } else {
        // Use cargo build for wasm32-wasip2 components
        println!("  Running cargo build...");
        let status = Command::new("cargo")
            .args(["build", "--release", "--target", "wasm32-wasip2"])
            .current_dir(&component.path)
            .status()
            .context("Failed to run cargo build")?;

        if !status.success() {
            anyhow::bail!("cargo build failed for {}", component.name);
        }
    }

    println!("  ✓ Built {}", component.name);

    Ok(())
}

pub fn build_components(only: Option<String>) -> Result<()> {
    // Find workspace root first
    let workspace_root = find_workspace_root()?;
    println!("Workspace root: {}", workspace_root.display());

    // Kill any processes on port 3000
    println!("Cleaning up processes on port 3000...");
    let _ = Command::new("lsof")
        .args(["-ti:3000"])
        .output()
        .and_then(|output| {
            if !output.stdout.is_empty() {
                let pids = String::from_utf8_lossy(&output.stdout);
                for pid in pids.trim().lines() {
                    let _ = Command::new("kill").args(["-9", pid]).status();
                }
            }
            Ok(())
        });

    let _ = Command::new("pkill").args(["-f", "spin up"]).status();

    // Discover components from workspace
    let components = get_components(&workspace_root)?;

    let to_build: Vec<&Component> = if let Some(only_list) = only {
        let names: Vec<&str> = only_list.split(',').map(|s| s.trim()).collect();
        components
            .iter()
            .filter(|c| names.contains(&c.name.as_str()))
            .collect()
    } else {
        components.iter().collect()
    };

    if to_build.is_empty() {
        println!("No components to build");
        return Ok(());
    }

    println!("Building {} component(s)...", to_build.len());

    for component in to_build {
        build_component(component)?;
    }

    println!("\n✓ All components built successfully!");

    Ok(())
}
