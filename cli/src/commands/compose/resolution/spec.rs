//! Component spec resolution
//!
//! This module handles resolving component specifications (aliases, paths, or
//! registry packages) to local file paths with cycle detection.

use anyhow::Result;
use std::collections::HashSet;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use crate::{commands::pkg, config};

use super::PackageClient;

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
    verbose: bool,
) -> Result<PathBuf> {
    resolve_component_spec_recursive(spec, deps_dir, client, &mut HashSet::new(), verbose).await
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
    verbose: bool,
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
            if verbose {
                println!("      Resolved alias '{}' → '{}'", spec, target);
            }
            return resolve_component_spec_recursive(target, deps_dir, client, visited, verbose)
                .await;
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
        if verbose {
            println!("      Downloading {} from registry...", spec);
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use tempfile::TempDir;

    /// Test circular alias detection
    #[test]
    fn test_circular_alias_detection() {
        let mut visited = HashSet::new();
        visited.insert("alias1".to_string());
        visited.insert("alias2".to_string());

        // Simulate detecting a cycle
        let spec = "alias1"; // Trying to visit alias1 again
        assert!(visited.contains(spec));

        // Verify error message format
        let chain: Vec<_> = visited.iter().map(|s| s.as_str()).collect();
        let error_msg = format!(
            "circular alias detected: {} → '{}'",
            chain.join(" → "),
            spec
        );
        assert!(error_msg.contains("circular alias detected"));
        assert!(error_msg.contains("alias1"));
    }

    /// Test format_resolution_error with single-step resolution
    #[test]
    fn test_format_resolution_error_simple() {
        let visited = HashSet::from(["component.wasm".to_string()]);
        let error = format_resolution_error(
            "component not found",
            "component.wasm",
            &visited,
            "file does not exist",
        );

        assert!(error.contains("component not found"));
        assert!(error.contains("component.wasm"));
        assert!(error.contains("file does not exist"));
        // Should NOT contain "resolution chain" for single-step
        assert!(!error.contains("resolution chain"));
    }

    /// Test format_resolution_error with multi-step resolution chain
    #[test]
    fn test_format_resolution_error_with_chain() {
        let visited = HashSet::from([
            "alias1".to_string(),
            "alias2".to_string(),
            "final.wasm".to_string(),
        ]);
        let error = format_resolution_error(
            "failed to download component",
            "final.wasm",
            &visited,
            "network error",
        );

        assert!(error.contains("failed to download component"));
        assert!(error.contains("final.wasm"));
        assert!(error.contains("network error"));
        // SHOULD contain "resolution chain" for multi-step
        assert!(error.contains("resolution chain"));
    }

    /// Test path spec detection
    #[test]
    fn test_path_spec_detection() {
        use crate::config::utils::is_path_spec;

        // Paths should be detected
        assert!(is_path_spec("./component.wasm"));
        assert!(is_path_spec("../component.wasm"));
        assert!(is_path_spec("/abs/path/component.wasm"));
        assert!(is_path_spec("~/component.wasm"));
        assert!(is_path_spec("path/to/component.wasm"));

        // Registry specs should NOT be detected as paths
        assert!(!is_path_spec("wasmcp:calculator@0.1.0"));
        assert!(!is_path_spec("namespace:component"));
        assert!(!is_path_spec("simple-alias"));
    }

    /// Test that visited set prevents infinite loops
    #[test]
    fn test_visited_set_usage() {
        let mut visited = HashSet::new();

        // First visit should succeed
        assert!(!visited.contains("spec1"));
        visited.insert("spec1".to_string());
        assert!(visited.contains("spec1"));

        // Second visit to same spec should be detected
        assert!(visited.contains("spec1"));

        // Different spec should work
        assert!(!visited.contains("spec2"));
        visited.insert("spec2".to_string());
        assert!(visited.contains("spec2"));

        // Both should now be in the set
        assert_eq!(visited.len(), 2);
    }

    /// Test async function signature and error propagation
    #[tokio::test]
    async fn test_resolve_component_spec_async_signature() {
        // This test verifies the async function can be called
        // Real testing requires a PackageClient and config setup
        let temp_dir = TempDir::new().unwrap();

        // Test with a path that doesn't exist
        let client = pkg::create_default_client().await.unwrap();
        let result =
            resolve_component_spec("./nonexistent.wasm", temp_dir.path(), &client, false).await;

        // Should fail because file doesn't exist
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("component not found") || err_msg.contains("No such file"));
    }

    /// Test resolution error messages are helpful
    #[test]
    fn test_error_message_quality() {
        let visited = HashSet::from(["alias1".to_string(), "alias2".to_string()]);

        let error = format_resolution_error(
            "failed to download component",
            "namespace:component@1.0.0",
            &visited,
            "404 Not Found",
        );

        // Error should be informative
        assert!(error.contains("failed to download component"));
        assert!(error.contains("namespace:component@1.0.0"));
        assert!(error.contains("404 Not Found"));
        assert!(error.contains("resolution chain"));
        assert!(error.contains("alias1"));
        assert!(error.contains("alias2"));
    }

    /// Test recursive resolution structure
    #[tokio::test]
    async fn test_recursive_resolution_terminates() {
        // Verify the recursive function signature is correct
        // The function uses Pin<Box<dyn Future>> for recursion
        let temp_dir = TempDir::new().unwrap();
        let mut visited = HashSet::new();
        let client = pkg::create_default_client().await.unwrap();

        // Test with invalid path - should terminate with error
        let result = resolve_component_spec_recursive(
            "./invalid.wasm",
            temp_dir.path(),
            &client,
            &mut visited,
            false,
        )
        .await;

        assert!(result.is_err());
        // Visited set should contain the spec we tried
        assert!(visited.contains("./invalid.wasm"));
    }
}
