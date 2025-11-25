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

/// Resolve a service component with runtime-specific variants
///
/// Some services have runtime-specific variants (e.g., kv-store vs kv-store-d2).
/// This function handles the variant selection logic generically.
///
/// # Arguments
///
/// * `service_name` - Name of the service (e.g., "kv-store")
/// * `override_spec` - Optional custom component spec to use instead of default
/// * `resolver` - Version resolver for component versions
/// * `deps_dir` - Directory where dependencies are cached
/// * `client` - OCI package client for downloads (only used for overrides)
/// * `verbose` - Show resolution messages
/// * `runtime` - Target runtime environment
///
/// # Returns
///
/// Path to the resolved component file.
pub async fn resolve_service_with_runtime(
    service_name: &str,
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
            println!("\nUsing override {}: {}", service_name, spec);
        }
        return spec::resolve_component_spec(spec, deps_dir, client, verbose).await;
    }

    // Determine variant based on runtime
    let component_name = match service_name {
        "kv-store" => {
            // Draft2 is default, only wasmcloud/wasmtime use stable WASI
            match runtime {
                "wasmcloud" | "wasmtime" => "kv-store",
                _ => "kv-store-d2",
            }
        }
        // Future services with variants go here
        _ => service_name, // No variant, use as-is
    };

    dependencies::get_dependency_path(component_name, resolver, deps_dir)
}

/// Resolve kv-store component (legacy alias)
///
/// This function is maintained for backward compatibility.
/// New code should use resolve_service_with_runtime directly.
pub async fn resolve_kv_store_component(
    override_spec: Option<&str>,
    resolver: &VersionResolver,
    deps_dir: &Path,
    client: &PackageClient,
    verbose: bool,
    runtime: &str,
) -> Result<PathBuf> {
    resolve_service_with_runtime(
        "kv-store",
        override_spec,
        resolver,
        deps_dir,
        client,
        verbose,
        runtime,
    )
    .await
}
