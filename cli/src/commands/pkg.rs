//! Package downloading using wasm-pkg-client library
//!
//! Provides functionality to download WebAssembly components from registries
//! without requiring an external wkg executable.

use crate::config;
use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use wasm_pkg_client::{
    caching::{CachingClient, FileCache},
    Client,
};
use wasm_pkg_common::{
    config::{Config, CustomConfig, RegistryMapping},
    metadata::RegistryMetadata,
    package::PackageSpec,
};

/// Create a config with wasmcp namespace mapped to ghcr.io
///
/// This loads the user's global wasm-pkg config (for authentication and custom registries)
/// and adds the wasmcp namespace mapping on top.
pub async fn create_wasmcp_config() -> Result<Config> {
    // Start with user's global config (includes auth, custom registries, etc.)
    let mut config = Config::global_defaults()
        .await
        .context("Failed to load wasm-pkg configuration")?;

    // Add wasmcp namespace mapping to ghcr.io OCI registry
    let wasmcp_namespace = "wasmcp"
        .parse()
        .expect("BUG: Failed to parse hardcoded 'wasmcp' namespace");

    // Create RegistryMetadata using serde_json deserialization
    let metadata: RegistryMetadata = serde_json::from_value(serde_json::json!({
        "preferredProtocol": "oci",
        "oci": {
            "registry": "ghcr.io"
        }
    }))
    .map_err(|e| anyhow::anyhow!("internal error creating registry metadata: {}", e))?;

    config.set_namespace_registry(
        wasmcp_namespace,
        RegistryMapping::Custom(CustomConfig {
            registry: "ghcr.io"
                .parse()
                .expect("BUG: Failed to parse hardcoded 'ghcr.io' registry"),
            metadata,
        }),
    );

    Ok(config)
}

/// Initialize a caching client for package downloads
///
/// Uses the centralized cache directory from config::paths
pub async fn create_client(cache_dir: &Path) -> Result<CachingClient<FileCache>> {
    let config = create_wasmcp_config().await?;
    let cache = FileCache::new(cache_dir)
        .await
        .context("Failed to create package cache")?;
    let client = Client::new(config);
    Ok(CachingClient::new(Some(client), cache))
}

/// Initialize a caching client using the default wasmcp cache directory
///
/// This is a convenience wrapper around `create_client()` that uses the
/// wasmcp-specific cache directory (~/.config/wasmcp/cache).
///
/// Note: Some operations (like WIT dependency management) use the global
/// wasm-pkg cache (~/.cache/wasm-pkg) instead and should call `create_client()`
/// directly with that path.
pub async fn create_default_client() -> Result<CachingClient<FileCache>> {
    let cache_dir = config::get_cache_dir()?;
    create_client(&cache_dir).await
}

/// Download a package to the specified output path
///
/// The spec can be either:
/// - A local file path (contains / or \, or ends with .wasm)
/// - A package spec like "namespace:name@version"
pub async fn download_package(
    client: &CachingClient<FileCache>,
    spec: &str,
    output_path: &Path,
) -> Result<()> {
    // Parse package spec
    let package_spec: PackageSpec = spec
        .parse()
        .with_context(|| format!("Invalid package spec: {}", spec))?;

    let package = package_spec.package;

    // Resolve version (fetch latest if not specified)
    let version = match package_spec.version {
        Some(v) => v,
        None => {
            let versions = client
                .list_all_versions(&package)
                .await
                .context("Failed to list package versions")?;
            versions
                .into_iter()
                .filter_map(|vi| (!vi.yanked).then_some(vi.version))
                .max()
                .with_context(|| format!("No releases found for {}", package))?
        }
    };

    // Get release metadata
    let release = client
        .get_release(&package, &version)
        .await
        .with_context(|| format!("Failed to get release {}@{}", package, version))?;

    // Create parent directory if needed
    if let Some(parent) = output_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("Failed to create output directory")?;
    }

    // Stream content to file
    let mut stream = client
        .get_content(&package, &release)
        .await
        .with_context(|| format!("Failed to stream content for {}@{}", package, version))?;

    let mut file = tokio::fs::File::create(output_path)
        .await
        .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;

    while let Some(chunk) = stream.try_next().await? {
        file.write_all(&chunk).await?;
    }

    file.flush().await?;

    Ok(())
}

/// Resolve a handler spec to a local path
///
/// - If spec is a local path, validates it exists and returns it
/// - If spec is a package spec, downloads it and returns the path
pub async fn resolve_spec(
    spec: &str,
    client: &CachingClient<FileCache>,
    deps_dir: &Path,
) -> Result<PathBuf> {
    // Check if spec is a local path using centralized detection
    if config::utils::is_path_spec(spec) {
        return config::utils::canonicalize_path(spec);
    }

    // Otherwise, treat as package spec and download
    let filename = spec.replace([':', '/'], "_") + ".wasm";
    let output_path = deps_dir.join(&filename);

    // Download the package
    download_package(client, spec, &output_path).await?;

    Ok(output_path)
}

/// Download multiple packages in parallel
pub async fn download_packages(
    client: &CachingClient<FileCache>,
    specs: &[String],
    deps_dir: &Path,
) -> Result<()> {
    use futures_util::future;

    // Create output directory
    tokio::fs::create_dir_all(deps_dir)
        .await
        .context("Failed to create deps directory")?;

    // Download all packages in parallel
    let downloads: Vec<_> = specs
        .iter()
        .map(|spec| {
            let client = client.clone();
            let filename = spec.replace([':', '/'], "_") + ".wasm";
            let output_path = deps_dir.join(&filename);
            async move {
                println!("   Downloading {}...", spec);
                download_package(&client, spec, &output_path)
                    .await
                    .with_context(|| format!("Failed to download {}", spec))?;
                Ok::<_, anyhow::Error>(())
            }
        })
        .collect();

    future::try_join_all(downloads).await?;

    println!("   All dependencies downloaded");
    Ok(())
}

/// Fetch WIT dependencies for a project
///
/// This uses wit-deps library to fetch dependencies from GitHub URLs specified in wit/deps.toml
/// wit-deps correctly handles extracting WIT files from tarballs and creating the proper
/// directory structure.
///
/// If `update` is true, clears existing lock file to force re-resolution
pub async fn fetch_wit_dependencies(project_dir: &Path, update: bool) -> Result<()> {
    println!("📦 Downloading WIT dependencies...");

    let manifest_path = project_dir.join("wit/deps.toml");
    let lock_path = project_dir.join("wit/deps.lock");
    let deps_dir = project_dir.join("wit/deps");

    // If update flag is set, remove existing lock file to force re-fetch
    if update && lock_path.exists() {
        tokio::fs::remove_file(&lock_path)
            .await
            .context("Failed to remove deps.lock")?;
    }

    // Use wit-deps library to fetch dependencies
    wit_deps::update_path(&manifest_path, &lock_path, &deps_dir)
        .await
        .context("Failed to fetch WIT dependencies")?;

    println!("   WIT dependencies resolved");

    Ok(())
}
