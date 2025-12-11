use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, Item, Value};
use walkdir::WalkDir;

const BACKUP_SUFFIX: &str = ".backup";

/// Find all deps.toml files in the workspace
fn find_deps_files() -> Result<Vec<PathBuf>> {
    let mut deps_files = Vec::new();

    for entry in WalkDir::new(".")
        .follow_links(true)
        .max_depth(10)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.file_name() == Some(std::ffi::OsStr::new("deps.toml"))
            && path.parent().and_then(|p| p.file_name()) == Some(std::ffi::OsStr::new("wit"))
        {
            deps_files.push(path.to_path_buf());
        }
    }

    Ok(deps_files)
}

/// Convert a URL dependency to a local path dependency
fn url_to_local_path(url: &str, local_base: &Path) -> Option<PathBuf> {
    // Extract package name from URL
    // Examples:
    // "https://github.com/wasmcp/wasmcp/releases/download/mcp-v2025-06-18-v0.1.6/wasmcp-mcp-v2025-06-18-0.1.6-source.tar.gz"
    // -> extract "mcp-v2025-06-18"

    let local_path = if url.contains("/wasmcp/") {
        // Extract version/package name from wasmcp releases
        url.split("/download/")
            .nth(1)
            .and_then(|part| part.split('/').next())
            .map(|name| local_base.join(name))
    } else if url.contains("/wasi-") {
        // Handle WASI dependencies like wasi-io, wasi-http, etc.
        url.split("/WebAssembly/")
            .nth(1)
            .and_then(|repo| repo.split('/').next())
            .map(|name| local_base.join(name))
    } else {
        None
    };

    // Validate the local path exists
    if let Some(ref path) = local_path {
        if !path.exists() {
            eprintln!(
                "  Warning: Local path does not exist: {} (skipping)",
                path.display()
            );
            return None;
        }
    }

    local_path
}

/// Patch a single deps.toml file to use local paths
fn patch_deps_file(deps_path: &Path, local_base: &Path) -> Result<()> {
    let content = fs::read_to_string(deps_path)
        .with_context(|| format!("Failed to read {}", deps_path.display()))?;

    let mut doc = content
        .parse::<DocumentMut>()
        .with_context(|| format!("Failed to parse TOML at {}", deps_path.display()))?;

    let backup_path = deps_path.with_extension(format!("toml{}", BACKUP_SUFFIX));

    // Create backup if it doesn't exist
    if !backup_path.exists() {
        fs::write(&backup_path, &content)
            .with_context(|| format!("Failed to create backup at {}", backup_path.display()))?;
        println!("  Created backup: {}", backup_path.display());
    }

    let mut changed = false;

    // Iterate through all top-level keys
    for (key, value) in doc.iter_mut() {
        if let Item::Value(Value::String(url)) = value {
            let url_str = url.value();
            if url_str.starts_with("http://") || url_str.starts_with("https://") {
                if let Some(local_path) = url_to_local_path(url_str, local_base) {
                    // Calculate relative path from deps.toml to local path
                    let deps_dir = deps_path.parent().with_context(|| {
                        format!("deps.toml has no parent directory: {}", deps_path.display())
                    })?;
                    let relative_path = pathdiff::diff_paths(&local_path, deps_dir)
                        .unwrap_or_else(|| local_path.clone());

                    // Create inline table for path dependency
                    let mut inline_table = toml_edit::InlineTable::new();
                    inline_table.insert("path", relative_path.display().to_string().into());

                    *value = Item::Value(Value::InlineTable(inline_table));
                    changed = true;

                    println!("  {} -> local path: {}", key, relative_path.display());
                }
            }
        }
    }

    if changed {
        fs::write(deps_path, doc.to_string()).with_context(|| {
            format!(
                "Failed to write patched deps.toml to {}",
                deps_path.display()
            )
        })?;
    }

    Ok(())
}

/// Restore a single deps.toml from backup
fn restore_deps_file(deps_path: &Path) -> Result<()> {
    let backup_path = deps_path.with_extension(format!("toml{}", BACKUP_SUFFIX));

    if backup_path.exists() {
        let backup_content = fs::read_to_string(&backup_path)
            .with_context(|| format!("Failed to read backup from {}", backup_path.display()))?;

        fs::write(deps_path, backup_content)
            .with_context(|| format!("Failed to restore {} from backup", deps_path.display()))?;

        // Keep backup file for future restores instead of deleting
        println!("  Restored: {} (backup preserved)", deps_path.display());
    } else {
        println!("  No backup found for: {}", deps_path.display());
    }

    Ok(())
}

pub fn patch_to_local(local_base: &Path) -> Result<()> {
    println!("Patching deps.toml files to use local paths...");
    println!("Local base directory: {}", local_base.display());

    let deps_files = find_deps_files()?;

    if deps_files.is_empty() {
        println!("No deps.toml files found in workspace");
        return Ok(());
    }

    for deps_path in deps_files {
        println!("\nProcessing: {}", deps_path.display());
        patch_deps_file(&deps_path, local_base)?;
    }

    println!("\nDone! Run 'dev-tools deps restore' to revert changes.");

    Ok(())
}

pub fn restore_from_backup() -> Result<()> {
    println!("Restoring deps.toml files from backups...");

    let deps_files = find_deps_files()?;

    if deps_files.is_empty() {
        println!("No deps.toml files found in workspace");
        return Ok(());
    }

    for deps_path in deps_files {
        restore_deps_file(&deps_path)?;
    }

    println!("\nDone!");

    Ok(())
}

pub fn show_status() -> Result<()> {
    println!("deps.toml status:");

    let deps_files = find_deps_files()?;

    if deps_files.is_empty() {
        println!("No deps.toml files found in workspace");
        return Ok(());
    }

    for deps_path in deps_files {
        let backup_path = deps_path.with_extension(format!("toml{}", BACKUP_SUFFIX));
        let has_backup = backup_path.exists();

        let status = if has_backup { "PATCHED" } else { "original" };
        println!("  {} - {}", deps_path.display(), status);
    }

    Ok(())
}
