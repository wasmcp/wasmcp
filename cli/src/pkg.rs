//! Package downloading using wasm-pkg-client library
//!
//! Provides functionality to download WebAssembly components from registries
//! without requiring an external wkg executable.

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
    let wasmcp_namespace = "wasmcp".parse().unwrap();

    // Create RegistryMetadata using serde_json deserialization
    let metadata: RegistryMetadata = serde_json::from_value(serde_json::json!({
        "preferredProtocol": "oci",
        "oci": {
            "registry": "ghcr.io"
        }
    }))
    .expect("Failed to create registry metadata");

    config.set_namespace_registry(
        wasmcp_namespace,
        RegistryMapping::Custom(CustomConfig {
            registry: "ghcr.io".parse().unwrap(),
            metadata,
        }),
    );

    Ok(config)
}

/// Initialize a caching client for package downloads
pub async fn create_client(cache_dir: &Path) -> Result<CachingClient<FileCache>> {
    let config = create_wasmcp_config().await?;
    let cache = FileCache::new(cache_dir)
        .await
        .context("Failed to create package cache")?;
    let client = Client::new(config);
    Ok(CachingClient::new(Some(client), cache))
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
    // Check if spec is a local path
    if spec.contains('/') || spec.contains('\\') || spec.ends_with(".wasm") {
        let path = PathBuf::from(spec);
        if !path.exists() {
            anyhow::bail!("Component not found: {}", spec);
        }
        return Ok(path);
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

    println!("   ✅ All dependencies downloaded");
    Ok(())
}

/// Fetch WIT dependencies for a project
///
/// This uses wasm-pkg-core to fetch all transitive WIT dependencies
/// (same as `wkg wit fetch`) and writes them to wit/deps/
///
/// If `update` is true, clears existing lock file packages to force re-resolution
/// (same as `wkg wit update`)
pub async fn fetch_wit_dependencies(project_dir: &Path, update: bool) -> Result<()> {
    println!("📦 Downloading WIT dependencies...");

    // Save current dir and cd into project to make LockFile::load work
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(project_dir)?;

    // Use global cache directory (same as wkg)
    let cache_dir = dirs::cache_dir()
        .context("Failed to get cache directory")?
        .join("wasm-pkg");

    // Create cache directory
    tokio::fs::create_dir_all(&cache_dir)
        .await
        .context("Failed to create cache directory")?;

    // Create package client with wasmcp namespace configured
    let client = create_client(&cache_dir)
        .await
        .context("Failed to create package client")?;

    // Load wkg.toml config (if present) or use default
    // This allows users to override dependencies locally
    let wkg_config = wasm_pkg_core::config::Config::default();

    // Load or create lock file
    let mut lock_file = wasm_pkg_core::lock::LockFile::load(false)
        .await
        .context("Failed to load lock file")?;

    // If update flag is set, clear existing packages to force re-resolution
    if update {
        lock_file.packages.clear();
    }

    // Use wasm-pkg-core to fetch all transitive dependencies
    // This is exactly what `wkg wit fetch` does internally
    // Pass "wit" as relative path since we changed to project_dir
    wasm_pkg_core::wit::fetch_dependencies(
        &wkg_config,
        "wit",
        &mut lock_file,
        client,
        wasm_pkg_core::wit::OutputType::Wit,
    )
    .await
    .context("Failed to fetch WIT dependencies")?;

    // Write lock file
    lock_file
        .write()
        .await
        .context("Failed to write wkg.lock file")?;

    // Restore original directory
    std::env::set_current_dir(original_dir)?;

    println!("   ✅ WIT dependencies resolved");

    Ok(())
}
