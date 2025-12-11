use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Runtime;

/// Read component names from versions.toml
fn load_component_names() -> Result<Vec<String>> {
    let versions_path = PathBuf::from("./cli/versions.toml");
    let contents = std::fs::read_to_string(&versions_path)
        .with_context(|| format!("Failed to read {}", versions_path.display()))?;

    let versions: HashMap<String, toml::Value> =
        toml::from_str(&contents).context("Failed to parse versions.toml")?;

    let versions_table = versions
        .get("versions")
        .and_then(|v| v.as_table())
        .context("versions.toml missing [versions] section")?;

    let mut components = Vec::new();
    for key in versions_table.keys() {
        // Skip WIT package versions (like mcp-v20250618)
        if !key.contains("v202") && key != "mcp-v20250618" {
            components.push(key.clone());
        }
    }

    Ok(components)
}

/// Find the wasmcp binary in CLI build outputs
fn find_wasmcp_binary() -> Result<PathBuf> {
    let cli_target = PathBuf::from("./cli/target");

    if !cli_target.exists() {
        bail!("CLI not built - run 'cargo build --release --manifest-path cli/Cargo.toml' first");
    }

    // Check common build targets
    let candidates = [
        cli_target.join("release/wasmcp"),
        cli_target.join("aarch64-apple-darwin/release/wasmcp"),
        cli_target.join("x86_64-apple-darwin/release/wasmcp"),
        cli_target.join("x86_64-unknown-linux-gnu/release/wasmcp"),
        cli_target.join("aarch64-unknown-linux-gnu/release/wasmcp"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return Ok(candidate.clone());
        }
    }

    bail!(
        "Could not find wasmcp binary in cli/target - tried: {:?}",
        candidates
    );
}

pub fn compose_components(
    components: Vec<String>,
    output_path: &Path,
    force: bool,
    local_overrides: bool,
    extra_args: Vec<String>,
) -> Result<()> {
    println!("Composing components...");

    // Ensure output directory exists
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory: {}", parent.display()))?;
    }

    let wasmcp_bin = find_wasmcp_binary()?;
    let mut cmd = Command::new(&wasmcp_bin);

    // Base compose server command
    cmd.arg("compose").arg("server");

    // Add component paths
    for component in &components {
        cmd.arg(component);
    }

    // Output path
    cmd.arg("-o").arg(output_path);

    // Force flag
    if force {
        cmd.arg("--force");
    }

    // Add local overrides if requested
    if local_overrides {
        let component_names =
            load_component_names().context("Failed to load component names from versions.toml")?;

        for component in &component_names {
            // Convert component name (e.g., "server-io") to wasm filename (e.g., "server_io.wasm")
            let wasm_name = component.replace('-', "_");

            // Special case: kv-store uses draft2 variant for local builds
            let wasm_file = if component == "kv-store" {
                format!("{}-d2.wasm", wasm_name)
            } else {
                format!("{}.wasm", wasm_name)
            };

            let path = format!("./target/wasm32-wasip2/release/{}", wasm_file);
            let path_buf = PathBuf::from(&path);

            if !path_buf.exists() {
                bail!(
                    "Override file missing: {} - run 'dev-tools build' first",
                    path
                );
            }

            cmd.arg("--override").arg(format!("{}={}", component, path));
        }
    }

    // Add any extra arguments passed through
    for arg in &extra_args {
        cmd.arg(arg);
    }

    println!("Running: wasmcp compose server");
    if !components.is_empty() {
        println!("  Components: {}", components.join(" "));
    }
    if local_overrides {
        println!("  With local overrides");
    }

    let output = cmd.output().context("Failed to run wasmcp compose")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Try to parse component-N pattern and map back to actual component
        if stderr.contains("component-") {
            eprintln!("\n{}", stderr);
            eprintln!("\nComponent mapping:");
            for (idx, comp) in components.iter().enumerate() {
                eprintln!("  component-{} = {}", idx + 1, comp);
            }
            eprintln!();
        }

        anyhow::bail!("wasmcp compose failed - see error output above");
    }

    // Print stdout in case there are warnings
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }

    println!("✓ Composed to: {}", output_path.display());

    Ok(())
}

pub fn run_component(runtime: Runtime, wasm: &Path) -> Result<()> {
    match runtime {
        Runtime::Spin => run_with_spin(wasm),
        Runtime::Wasmtime => run_with_wasmtime(wasm),
    }
}

fn run_with_spin(wasm: &Path) -> Result<()> {
    println!("Running with Spin...");

    // Change to .agent directory where spin.toml should be
    let agent_dir = wasm.parent().unwrap_or_else(|| Path::new(".agent"));

    let status = Command::new("spin")
        .args(["up", "-f", "spin.toml"])
        .args([
            "-e",
            "WASMCP_SESSION_ENABLED=true",
            "-e",
            "WASMCP_SESSION_BUCKET=default",
        ])
        .current_dir(agent_dir)
        .status()
        .context("Failed to run spin")?;

    if !status.success() {
        anyhow::bail!("spin up failed");
    }

    Ok(())
}

fn run_with_wasmtime(_wasm: &Path) -> Result<()> {
    println!("Running with wasmtime...");
    // TODO: Implement wasmtime execution
    println!("Wasmtime support not yet implemented");
    Ok(())
}
