//! Profile resolution and component merging
//!
//! This module handles resolving profiles with base inheritance and merging
//! profile components with direct component specifications.

use anyhow::{bail, Result};
use std::collections::HashSet;

use crate::config;

/// Resolve a profile with base inheritance
///
/// Recursively resolves the profile chain, merging components and settings.
/// Base profile components come first, then the profile's own components.
pub fn resolve_profile(
    profile_name: &str,
    cfg: &config::WasmcpConfig,
    visited: &mut HashSet<String>,
) -> Result<config::Profile> {
    // Detect circular dependencies
    if !visited.insert(profile_name.to_string()) {
        bail!("circular profile dependency detected: '{}'", profile_name);
    }

    let profile = cfg
        .profiles
        .get(profile_name)
        .ok_or_else(|| anyhow::anyhow!("profile '{}' not found", profile_name))?;

    // If profile has a base, resolve it first
    if let Some(base_name) = &profile.base {
        let mut base = resolve_profile(base_name, cfg, visited)?;

        // Merge: base components + profile components (order matters!)
        // Use extend with iterator to avoid unnecessary intermediate allocations
        base.components.extend(profile.components.iter().cloned());

        // Profile settings override base settings
        base.output.clone_from(&profile.output);

        Ok(base)
    } else {
        // Clone only at leaf nodes
        Ok(profile.clone())
    }
}

/// Expand components that might be profiles into their constituent components
///
/// This function checks each spec in order:
/// 1. If it's a profile name, expand it in-place to its components
/// 2. Otherwise, keep it as-is (will be resolved as alias/path/registry later)
///
/// This maintains the order: if you pass [comp1, profile, comp2] and profile
/// contains [p1, p2], the result is [comp1, p1, p2, comp2].
///
/// Returns the expanded component list and the last profile's settings (if any).
pub fn expand_profile_specs(specs: &[String]) -> Result<(Vec<String>, Option<config::Profile>)> {
    let cfg = config::load_config()?;
    let mut expanded = Vec::new();
    let mut last_profile: Option<config::Profile> = None;

    for spec in specs {
        // Check if this spec is a profile name
        if cfg.profiles.contains_key(spec) {
            let mut visited = HashSet::new();
            let profile = resolve_profile(spec, &cfg, &mut visited)?;

            // Expand profile's components in-place
            expanded.extend(profile.components.iter().cloned());

            // Track the last profile for settings (output path)
            last_profile = Some(profile);
        } else {
            // Not a profile, keep as-is (will be resolved as alias/path/registry)
            expanded.push(spec.clone());
        }
    }

    if expanded.is_empty() {
        bail!("no components specified");
    }

    Ok((expanded, last_profile))
}

/// Compose profiles and direct components into a single component list
///
/// Resolution order:
/// 1. Each profile is resolved (with base inheritance) in order
/// 2. All profile components are collected
/// 3. Direct components from CLI are appended
///
/// Profile settings behavior:
/// - When multiple profiles are specified, the last profile's settings are used
/// - This means the last profile's output path becomes the default (unless -o is provided)
/// - Component lists are concatenated in order (all profiles are merged)
pub fn compose_profiles_and_components(
    profile_names: &[String],
    direct_components: &[String],
) -> Result<(Vec<String>, Option<config::Profile>)> {
    let cfg = config::load_config()?;
    let mut all_components = Vec::new();
    let mut merged_profile: Option<config::Profile> = None;

    // Step 1: Resolve and merge all profiles
    for name in profile_names {
        let mut visited = HashSet::new();
        let profile = resolve_profile(name, &cfg, &mut visited)?;

        // Collect components using iterator to avoid unnecessary intermediate clones
        all_components.extend(profile.components.iter().cloned());

        // Merge profile settings (last profile wins)
        merged_profile = Some(profile);
    }

    // Step 2: Append direct component specs from CLI
    all_components.extend(direct_components.iter().cloned());

    if all_components.is_empty() {
        bail!("no components specified in profiles or command line");
    }

    Ok((all_components, merged_profile))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_profile(components: Vec<String>, base: Option<String>) -> config::Profile {
        config::Profile {
            components,
            base,
            output: "test-output.wasm".to_string(),
        }
    }

    fn create_test_config(profiles: HashMap<String, config::Profile>) -> config::WasmcpConfig {
        config::WasmcpConfig {
            components: HashMap::new(),
            profiles,
        }
    }

    #[test]
    fn test_resolve_profile_simple() {
        let mut profiles = HashMap::new();
        profiles.insert(
            "test".to_string(),
            create_test_profile(vec!["comp1".to_string(), "comp2".to_string()], None),
        );
        let cfg = create_test_config(profiles);

        let mut visited = HashSet::new();
        let result = resolve_profile("test", &cfg, &mut visited).unwrap();

        assert_eq!(result.components, vec!["comp1", "comp2"]);
        assert_eq!(visited.len(), 1);
        assert!(visited.contains("test"));
    }

    #[test]
    fn test_resolve_profile_with_base() {
        let mut profiles = HashMap::new();
        profiles.insert(
            "base".to_string(),
            create_test_profile(vec!["base1".to_string(), "base2".to_string()], None),
        );
        profiles.insert(
            "derived".to_string(),
            create_test_profile(
                vec!["derived1".to_string()],
                Some("base".to_string()),
            ),
        );
        let cfg = create_test_config(profiles);

        let mut visited = HashSet::new();
        let result = resolve_profile("derived", &cfg, &mut visited).unwrap();

        // Base components should come first, then derived
        assert_eq!(result.components, vec!["base1", "base2", "derived1"]);
    }

    #[test]
    fn test_resolve_profile_circular_dependency() {
        let mut profiles = HashMap::new();
        profiles.insert(
            "profile-a".to_string(),
            create_test_profile(vec!["a".to_string()], Some("profile-b".to_string())),
        );
        profiles.insert(
            "profile-b".to_string(),
            create_test_profile(vec!["b".to_string()], Some("profile-a".to_string())),
        );
        let cfg = create_test_config(profiles);

        let mut visited = HashSet::new();
        let result = resolve_profile("profile-a", &cfg, &mut visited);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("circular"));
    }

    #[test]
    fn test_resolve_profile_self_reference() {
        let mut profiles = HashMap::new();
        profiles.insert(
            "self-ref".to_string(),
            create_test_profile(vec!["comp".to_string()], Some("self-ref".to_string())),
        );
        let cfg = create_test_config(profiles);

        let mut visited = HashSet::new();
        let result = resolve_profile("self-ref", &cfg, &mut visited);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("circular"));
    }

    #[test]
    fn test_resolve_profile_not_found() {
        let cfg = create_test_config(HashMap::new());

        let mut visited = HashSet::new();
        let result = resolve_profile("nonexistent", &cfg, &mut visited);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_resolve_profile_chain() {
        let mut profiles = HashMap::new();
        profiles.insert(
            "level1".to_string(),
            create_test_profile(vec!["l1".to_string()], None),
        );
        profiles.insert(
            "level2".to_string(),
            create_test_profile(vec!["l2".to_string()], Some("level1".to_string())),
        );
        profiles.insert(
            "level3".to_string(),
            create_test_profile(vec!["l3".to_string()], Some("level2".to_string())),
        );
        let cfg = create_test_config(profiles);

        let mut visited = HashSet::new();
        let result = resolve_profile("level3", &cfg, &mut visited).unwrap();

        // Components should be ordered: l1, l2, l3
        assert_eq!(result.components, vec!["l1", "l2", "l3"]);
    }

    #[test]
    fn test_compose_profiles_and_components_no_profiles() {
        let result = compose_profiles_and_components(
            &[],
            &["comp1".to_string(), "comp2".to_string()],
        )
        .unwrap();

        assert_eq!(result.0, vec!["comp1", "comp2"]);
        assert!(result.1.is_none());
    }

    #[test]
    fn test_compose_profiles_and_components_empty() {
        let result = compose_profiles_and_components(&[], &[]);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("no components specified"));
    }
}
