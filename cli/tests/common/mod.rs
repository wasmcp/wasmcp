//! Common test utilities and fixtures
//!
//! This module provides shared test helpers for wasmcp CLI tests.

use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Creates a temporary directory for test fixtures
pub fn create_temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

/// Get the path to a test fixture WASM component
///
/// This function looks for components in:
/// 1. tests/fixtures/ (pre-built test components)
/// 2. ../examples/ (example components from the repository)
pub fn get_fixture_path(name: &str) -> Option<PathBuf> {
    // Try tests/fixtures/ first
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);

    if fixture_path.exists() {
        return Some(fixture_path);
    }

    // Try examples directory (from repository root)
    let examples_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Should have parent directory")
        .join("examples")
        .join(name);

    if examples_path.exists() {
        return Some(examples_path);
    }

    None
}

/// Get the path to the calculator example component
pub fn get_calculator_wasm() -> Option<PathBuf> {
    get_fixture_path("calculator-rs/target/wasm32-wasip2/release/calculator.wasm")
}

/// Get the path to the strings example component
pub fn get_strings_wasm() -> Option<PathBuf> {
    get_fixture_path("strings-py/strings.wasm")
}

/// Creates a test configuration file with components
pub fn create_test_config(temp_dir: &Path, components: &[(&str, &str)]) -> std::io::Result<PathBuf> {
    let config_path = temp_dir.join("config.toml");
    let mut content = String::from("[components]\n");

    for (name, path) in components {
        content.push_str(&format!("{} = \"{}\"\n", name, path));
    }

    std::fs::write(&config_path, content)?;
    Ok(config_path)
}

/// Creates a test configuration file with profiles
pub fn create_test_config_with_profiles(
    temp_dir: &Path,
    components: &[(&str, &str)],
    profiles: &[(&str, &[&str], &str)], // (name, components, output)
) -> std::io::Result<PathBuf> {
    let config_path = temp_dir.join("config.toml");
    let mut content = String::from("[components]\n");

    for (name, path) in components {
        content.push_str(&format!("{} = \"{}\"\n", name, path));
    }

    content.push_str("\n");

    for (name, components, output) in profiles {
        content.push_str("[[profiles]]\n");
        content.push_str(&format!("name = \"{}\"\n", name));
        content.push_str(&format!("components = [{}]\n",
            components.iter()
                .map(|c| format!("\"{}\"", c))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        content.push_str(&format!("output = \"{}\"\n\n", output));
    }

    std::fs::write(&config_path, content)?;
    Ok(config_path)
}
