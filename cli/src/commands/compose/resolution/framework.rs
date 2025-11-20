//! Framework component resolution
//!
//! This module handles resolution and downloading of wasmcp framework components
//! (transport, server-io, session-store, method-not-found). Framework components are
//! downloaded from OCI registries and cached locally.
//!
//! # Resolution Flow
//!
//! 1. Check if override spec provided (custom component)
//! 2. If no override, ensure framework component is downloaded
//! 3. Return path to local component file

use anyhow::Result;
use std::path::{Path, PathBuf};

use super::{PackageClient, dependencies, spec};
use crate::commands::compose::inspection::interfaces::ComponentType;
use crate::versioning::VersionResolver;

/// Generic resolver for framework components
///
/// Resolves a framework component to a local file path, either by using an
/// override spec or by getting the path to the already-downloaded default component.
///
/// # Arguments
///
/// * `component_name` - Name of framework component to resolve (e.g., "transport", "server-io")
/// * `override_spec` - Optional custom component spec to use instead of default
/// * `resolver` - Version resolver for component versions
/// * `deps_dir` - Directory where dependencies are cached
/// * `client` - OCI package client for downloads (only used for overrides)
/// * `verbose` - Show resolution messages
///
/// # Returns
///
/// Path to the resolved component file.
///
/// # Errors
///
/// Returns an error if:
/// - Override spec cannot be resolved
/// - Component file not found (call download_dependencies first)
pub async fn resolve_framework_component(
    component_name: &str,
    override_spec: Option<&str>,
    resolver: &VersionResolver,
    deps_dir: &Path,
    client: &PackageClient,
    verbose: bool,
) -> Result<PathBuf> {
    if let Some(spec) = override_spec {
        if verbose {
            println!("\nUsing override {}: {}", component_name, spec);
        }
        spec::resolve_component_spec(spec, deps_dir, client, verbose).await
    } else {
        dependencies::get_dependency_path(component_name, resolver, deps_dir)
    }
}

/// Resolve kv-store component (override or default)
pub async fn resolve_kv_store_component(
    override_spec: Option<&str>,
    resolver: &VersionResolver,
    deps_dir: &Path,
    client: &PackageClient,
    verbose: bool,
    runtime: &str,
) -> Result<PathBuf> {
    // If override provided, use it
    if let Some(spec) = override_spec {
        if verbose {
            println!("\nUsing override kv-store: {}", spec);
        }
        return spec::resolve_component_spec(spec, deps_dir, client, verbose).await;
    }

    // Determine package name based on runtime
    // "spin" uses kv-store-d2 (draft 2 WASI)
    // "wasmcloud" and "wasmtime" use kv-store (standard WASI)
    let component_name = match runtime {
        "spin" => "kv-store-d2",
        "wasmcloud" | "wasmtime" => ComponentType::KvStore.name(),
        _ => anyhow::bail!(
            "unsupported runtime: '{}' (must be 'spin', 'wasmcloud', or 'wasmtime')",
            runtime
        ),
    };

    dependencies::get_dependency_path(component_name, resolver, deps_dir)
}

