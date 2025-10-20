//! Component spec resolution
//!
//! This module handles resolving component specifications (aliases, paths, or
//! registry packages) to local file paths with cycle detection.

use anyhow::Result;
use std::collections::HashSet;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use crate::{config, pkg};

use super::dependencies::PackageClient;

/// Resolve a component spec (alias, path, or package spec) to a local file path
///
/// Resolution order:
/// 1. Check if spec is an alias in config → recursively resolve target
/// 2. Check if spec is a local path (contains /, \, or ends with .wasm) → validate existence
/// 3. Treat as package spec → download from registry
///
/// # Examples
///
/// ```text
/// calc                           → (alias) → wasmcp:calculator@0.1.0 → deps/...
/// ./my-handler.wasm              → ./my-handler.wasm (if exists)
/// /abs/path/handler.wasm         → /abs/path/handler.wasm (if exists)
/// wasmcp:calculator@0.1.0        → deps/wasmcp_calculator@0.1.0.wasm
/// namespace:name                 → deps/namespace_name@latest.wasm
/// ```
pub async fn resolve_component_spec(
    spec: &str,
    deps_dir: &Path,
    client: &PackageClient,
) -> Result<PathBuf> {
    resolve_component_spec_recursive(spec, deps_dir, client, &mut HashSet::new()).await
}

/// Format a resolution error with optional chain context
///
/// If the resolution chain has multiple steps, includes the full chain in the error.
/// Otherwise, returns a simpler error message.
fn format_resolution_error(
    operation: &str,
    spec: &str,
    visited: &HashSet<String>,
    error: impl std::fmt::Display,
) -> String {
    if visited.len() > 1 {
        // Only allocate chain Vec when needed
        let chain: Vec<_> = visited.iter().map(|s| s.as_str()).collect();
        format!(
            "{}: '{}'\nresolution chain: {}\nerror: {}",
            operation,
            spec,
            chain.join(" → "),
            error
        )
    } else {
        format!("{}: '{}'\nerror: {}", operation, spec, error)
    }
}

/// Internal recursive resolver with cycle detection
fn resolve_component_spec_recursive<'a>(
    spec: &'a str,
    deps_dir: &'a Path,
    client: &'a PackageClient,
    visited: &'a mut HashSet<String>,
) -> Pin<Box<dyn Future<Output = Result<PathBuf>> + 'a>> {
    Box::pin(async move {
        // Detect circular aliases
        if visited.contains(spec) {
            let chain: Vec<_> = visited.iter().map(|s| s.as_str()).collect();
            anyhow::bail!(
                "circular alias detected: {} → '{}'",
                chain.join(" → "),
                spec
            );
        }
        visited.insert(spec.to_string());

        // 1. Check if spec is an alias in config
        let config = config::load_config()?;
        if let Some(target) = config.components.get(spec) {
            println!("      Resolved alias '{}' → '{}'", spec, target);
            return resolve_component_spec_recursive(target, deps_dir, client, visited).await;
        }

        // 2. Check if spec looks like a file path
        if config::utils::is_path_spec(spec) {
            // Try to canonicalize the path (validates existence and resolves to absolute)
            match config::utils::canonicalize_path(spec) {
                Ok(path) => return Ok(path),
                Err(e) => {
                    anyhow::bail!(format_resolution_error(
                        "component not found",
                        spec,
                        visited,
                        e
                    ));
                }
            }
        }

        // 3. Otherwise treat as package spec and download
        println!("      Downloading {} from registry...", spec);
        pkg::resolve_spec(spec, client, deps_dir)
            .await
            .map_err(|e| {
                anyhow::anyhow!(format_resolution_error(
                    "failed to download component",
                    spec,
                    visited,
                    e
                ))
            })
    })
}
