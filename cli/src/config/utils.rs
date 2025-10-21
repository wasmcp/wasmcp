//! Utility functions for configuration management
//!
//! This module provides shared helper functions for component spec detection
//! and path resolution, centralizing logic that was previously duplicated across
//! multiple modules.
//!
//! ## Component Spec Classification
//!
//! The wasmcp CLI accepts component specifications in three forms:
//!
//! 1. **Registry packages** (e.g., `wasmcp:calculator@0.1.0`)
//!    - Contain a colon `:` to separate namespace and name
//!    - Optionally include version with `@version`
//!    - Downloaded from OCI registries
//!
//! 2. **File paths** (e.g., `./handler.wasm`, `~/project/handler.wasm`)
//!    - Have explicit path indicators: `./`, `../`, `~/`, absolute paths
//!    - Contain path separators: `/` or `\`
//!    - End with `.wasm` extension
//!    - Resolved to absolute paths and validated for existence
//!
//! 3. **Aliases** (e.g., `calc`, `my-handler`)
//!    - Simple identifiers that don't match registry or path patterns
//!    - Resolved through the configuration file's component registry
//!    - Can point to registry packages, file paths, or other aliases
//!
//! ## Detection Strategy
//!
//! The classification uses a multi-stage approach implemented in [`is_path_spec()`]
//! and [`is_registry_spec()`]:
//!
//! ```text
//! ┌─────────────────────────┐
//! │   Component Spec        │
//! └───────────┬─────────────┘
//!             │
//!             ▼
//!     ┌───────────────┐
//!     │ Contains ':'? │──Yes──> Registry Package
//!     └───────┬───────┘
//!             │ No
//!             ▼
//!     ┌───────────────────┐
//!     │ Path indicators?  │──Yes──> File Path
//!     │ (./, ~/, /, \)    │
//!     └───────┬───────────┘
//!             │ No
//!             ▼
//!     ┌───────────────────┐
//!     │ Contains / or \?  │──Yes──> File Path
//!     └───────┬───────────┘
//!             │ No
//!             ▼
//!     ┌───────────────────┐
//!     │ Ends with .wasm?  │──Yes──> File Path
//!     └───────┬───────────┘
//!             │ No
//!             ▼
//!          Alias
//! ```
//!
//! ## Edge Cases and Design Decisions
//!
//! ### Registry Packages with Path-like Names
//!
//! Registry specs are checked first, so `namespace:name.wasm@1.0` is correctly
//! identified as a registry package despite containing `.wasm`.
//!
//! ### Files Without Extensions
//!
//! Files must have either a path separator or the `.wasm` extension to be
//! detected as paths. A file named just `handler` in the current directory
//! would be treated as an alias unless referenced as `./handler`.
//!
//! ### Symlinks
//!
//! Path canonicalization follows symlinks to their targets. This supports
//! development workflows where components are symlinked but requires users
//! to be aware that the resolved path may differ from the input path.
//!
//! ## Examples
//!
//! ```rust
//! use wasmcp::config::utils;
//!
//! // Registry packages
//! assert!(utils::is_registry_spec("wasmcp:calculator@0.1.0"));
//! assert!(utils::is_registry_spec("my-org:handler"));
//!
//! // File paths
//! assert!(utils::is_path_spec("./handler.wasm"));
//! assert!(utils::is_path_spec("../target/handler.wasm"));
//! assert!(utils::is_path_spec("~/projects/handler.wasm"));
//! assert!(utils::is_path_spec("/abs/path/handler.wasm"));
//! assert!(utils::is_path_spec("handler.wasm"));
//!
//! // Aliases (neither registry nor path)
//! assert!(!utils::is_registry_spec("calc"));
//! assert!(!utils::is_path_spec("calc"));
//! assert!(!utils::is_registry_spec("my-handler"));
//! assert!(!utils::is_path_spec("my-handler"));
//! ```

use std::path::PathBuf;

/// Determine if a spec looks like a local file path
///
/// A spec is considered a path if it:
/// - Contains path separators (/ or \)
/// - Ends with .wasm extension
/// - Starts with ./ or ../ (relative path indicators)
/// - Starts with ~/ (home directory)
///
/// Otherwise it's assumed to be either:
/// - A registry package spec (namespace:name@version)
/// - An alias to another component
///
/// # Examples
///
/// ```
/// use wasmcp::config::utils::is_path_spec;
///
/// assert!(is_path_spec("./handler.wasm"));
/// assert!(is_path_spec("../target/handler.wasm"));
/// assert!(is_path_spec("/abs/path/handler.wasm"));
/// assert!(is_path_spec("~/handler.wasm"));
/// assert!(is_path_spec("handler.wasm"));
/// assert!(!is_path_spec("wasmcp:calculator@0.1.0"));
/// assert!(!is_path_spec("calc")); // Could be an alias
/// ```
pub fn is_path_spec(spec: &str) -> bool {
    // Explicit path indicators
    if spec.starts_with("./")
        || spec.starts_with("../")
        || spec.starts_with("~/")
        || spec.starts_with('/')
        || spec.starts_with('\\')
    {
        return true;
    }

    // Path separators anywhere in the spec
    if spec.contains('/') || spec.contains('\\') {
        return true;
    }

    // .wasm extension (even without path separators, treat as local file)
    if spec.ends_with(".wasm") {
        return true;
    }

    false
}

/// Determine if a spec looks like a registry package spec
///
/// A spec is considered a registry package if it:
/// - Contains a colon (indicating namespace:name format)
/// - Optionally has @version suffix
///
/// # Examples
///
/// ```
/// use wasmcp::config::utils::is_registry_spec;
///
/// assert!(is_registry_spec("wasmcp:calculator@0.1.0"));
/// assert!(is_registry_spec("namespace:name"));
/// assert!(!is_registry_spec("./handler.wasm"));
/// assert!(!is_registry_spec("calc")); // Could be an alias
/// ```
pub fn is_registry_spec(spec: &str) -> bool {
    spec.contains(':')
}

/// Canonicalize a path spec to an absolute path
///
/// This function:
/// 1. Expands ~ to home directory
/// 2. Resolves relative paths (. and ..)
/// 3. Converts to absolute path
/// 4. Validates that the path exists
///
/// # Security Considerations
///
/// This function follows symlinks during canonicalization. This is intentional behavior
/// to support common workflows where components are symlinked (e.g., during development).
/// The function validates that the final resolved path exists and is readable.
///
/// Users should be aware that:
/// - Symlinks are followed to their target
/// - The resolved path may be outside the original directory
/// - Path traversal (../) is resolved to the actual filesystem location
///
/// This behavior is standard for CLI tools that work with the filesystem.
///
/// # Errors
///
/// Returns an error if:
/// - The path doesn't exist
/// - The path cannot be canonicalized (permissions, broken symlink, etc.)
/// - Home directory expansion fails (for ~ paths)
pub fn canonicalize_path(spec: &str) -> anyhow::Result<PathBuf> {
    let path = if let Some(stripped) = spec.strip_prefix("~/") {
        // Expand ~ to home directory
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE")) // Windows fallback
            .map_err(|_| anyhow::anyhow!("cannot determine home directory"))?;
        PathBuf::from(home).join(stripped)
    } else {
        PathBuf::from(spec)
    };

    // Canonicalize to get absolute path and validate existence
    path.canonicalize().map_err(|e| {
        anyhow::anyhow!(
            "path does not exist or cannot be resolved: '{}'\nerror: {}",
            spec,
            e
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_path_spec() {
        // Paths with explicit indicators
        assert!(is_path_spec("./handler.wasm"));
        assert!(is_path_spec("../target/handler.wasm"));
        assert!(is_path_spec("/abs/path/handler.wasm"));
        assert!(is_path_spec("~/handler.wasm"));

        // Paths with separators
        assert!(is_path_spec("target/debug/handler.wasm"));
        assert!(is_path_spec("C:\\Windows\\handler.wasm"));

        // .wasm extension
        assert!(is_path_spec("handler.wasm"));

        // Not paths - registry specs
        assert!(!is_path_spec("wasmcp:calculator@0.1.0"));
        assert!(!is_path_spec("namespace:name"));

        // Not paths - aliases
        assert!(!is_path_spec("calc"));
        assert!(!is_path_spec("my-handler"));
    }

    #[test]
    fn test_is_registry_spec() {
        assert!(is_registry_spec("wasmcp:calculator@0.1.0"));
        assert!(is_registry_spec("namespace:name"));
        assert!(is_registry_spec("a:b"));

        assert!(!is_registry_spec("./handler.wasm"));
        assert!(!is_registry_spec("calc"));
        assert!(!is_registry_spec("my-handler"));
    }

    #[test]
    fn test_canonicalize_path_errors() {
        // Non-existent path should error
        let result = canonicalize_path("/this/path/definitely/does/not/exist.wasm");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("does not exist or cannot be resolved")
        );
    }

    #[test]
    fn test_canonicalize_path_with_tempfile() {
        // Create a temporary file to test actual canonicalization
        let temp_file = std::env::temp_dir().join("wasmcp-test-handler.wasm");
        std::fs::write(&temp_file, b"test").unwrap();

        // Test absolute path
        let result = canonicalize_path(temp_file.to_str().unwrap());
        assert!(result.is_ok());
        let canonical = result.unwrap();
        assert!(canonical.is_absolute());
        assert!(canonical.ends_with("wasmcp-test-handler.wasm"));

        // Cleanup
        std::fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_home_directory_expansion() {
        // We can't actually test home directory expansion without creating
        // a file in the user's home directory, but we can test the logic
        // by checking if a tilde path is recognized
        assert!(is_path_spec("~/handler.wasm"));
        assert!(is_path_spec("~/projects/target/handler.wasm"));
    }

    #[test]
    fn test_registry_spec_with_versions() {
        // Test various version formats
        assert!(is_registry_spec("wasmcp:calculator@0.1.0"));
        assert!(is_registry_spec("namespace:name@^1.2.3"));
        assert!(is_registry_spec("foo:bar@~2.0"));
        assert!(is_registry_spec("org:pkg@1.0.0-beta.1"));

        // Test without version
        assert!(is_registry_spec("wasmcp:calculator"));
        assert!(is_registry_spec("org:handler"));
    }

    #[test]
    fn test_edge_case_registry_with_wasm_in_name() {
        // Edge case: registry package with .wasm in the name
        // Registry spec check happens first, so this should be registry, not path
        assert!(is_registry_spec("namespace:handler.wasm@1.0"));
        assert!(!is_path_spec("namespace:handler.wasm@1.0"));
    }

    #[test]
    fn test_relative_path_detection() {
        // Various relative path formats
        assert!(is_path_spec("./handler.wasm"));
        assert!(is_path_spec("../handler.wasm"));
        assert!(is_path_spec("../../target/handler.wasm"));
        assert!(is_path_spec("./target/debug/handler.wasm"));
    }
}
