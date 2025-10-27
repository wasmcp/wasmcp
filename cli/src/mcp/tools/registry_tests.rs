//! Tests for registry tools module
//!
//! Separated into its own file to avoid string matching issues with Edit tool

use super::registry::*;

/// Test RegistryListArgs default target
#[test]
fn test_registry_list_args_default() {
    let json = serde_json::json!({});
    let args: RegistryListArgs = serde_json::from_value(json).unwrap();
    assert_eq!(args.target, "all");
}

/// Test RegistryListArgs with explicit target
#[test]
fn test_registry_list_args_targets() {
    let components = serde_json::json!({"target": "components"});
    let args: RegistryListArgs = serde_json::from_value(components).unwrap();
    assert_eq!(args.target, "components");

    let profiles = serde_json::json!({"target": "profiles"});
    let args: RegistryListArgs = serde_json::from_value(profiles).unwrap();
    assert_eq!(args.target, "profiles");

    let all = serde_json::json!({"target": "all"});
    let args: RegistryListArgs = serde_json::from_value(all).unwrap();
    assert_eq!(args.target, "all");
}

/// Test AddComponentArgs deserialization
#[test]
fn test_add_component_args() {
    let json = serde_json::json!({
        "alias": "calc",
        "spec": "./calculator.wasm"
    });

    let args: AddComponentArgs = serde_json::from_value(json).unwrap();
    assert_eq!(args.alias, "calc");
    assert_eq!(args.spec, "./calculator.wasm");
}

/// Test AddComponentArgs with registry spec
#[test]
fn test_add_component_args_registry_spec() {
    let json = serde_json::json!({
        "alias": "math",
        "spec": "wasmcp:math@0.1.0"
    });

    let args: AddComponentArgs = serde_json::from_value(json).unwrap();
    assert_eq!(args.alias, "math");
    assert_eq!(args.spec, "wasmcp:math@0.1.0");
}

/// Test AddComponentArgs missing required fields
#[test]
fn test_add_component_args_missing_fields() {
    let missing_alias = serde_json::json!({"spec": "./test.wasm"});
    assert!(serde_json::from_value::<AddComponentArgs>(missing_alias).is_err());

    let missing_spec = serde_json::json!({"alias": "test"});
    assert!(serde_json::from_value::<AddComponentArgs>(missing_spec).is_err());
}

/// Test AddProfileArgs with all fields
#[test]
fn test_add_profile_args_complete() {
    let json = serde_json::json!({
        "name": "dev",
        "components": ["calc", "weather"],
        "output": "dev-server.wasm"
    });

    let args: AddProfileArgs = serde_json::from_value(json).unwrap();
    assert_eq!(args.name, "dev");
    assert_eq!(args.components, vec!["calc", "weather"]);
    assert_eq!(args.output, Some("dev-server.wasm".to_string()));
}

/// Test AddProfileArgs without output
#[test]
fn test_add_profile_args_no_output() {
    let json = serde_json::json!({
        "name": "test",
        "components": ["component1"]
    });

    let args: AddProfileArgs = serde_json::from_value(json).unwrap();
    assert_eq!(args.name, "test");
    assert_eq!(args.components, vec!["component1"]);
    assert_eq!(args.output, None);
}

/// Test AddProfileArgs with empty components
#[test]
fn test_add_profile_args_empty_components() {
    let json = serde_json::json!({
        "name": "empty",
        "components": []
    });

    let args: AddProfileArgs = serde_json::from_value(json).unwrap();
    assert_eq!(args.name, "empty");
    assert!(args.components.is_empty());
}

/// Test AddProfileArgs with multiple components
#[test]
fn test_add_profile_args_multiple_components() {
    let json = serde_json::json!({
        "name": "full",
        "components": ["comp1", "comp2", "comp3", "comp4"]
    });

    let args: AddProfileArgs = serde_json::from_value(json).unwrap();
    assert_eq!(args.components.len(), 4);
}

/// Test RemoveArgs for component removal
#[test]
fn test_remove_args_component() {
    let json = serde_json::json!({
        "kind": "component",
        "name": "calc"
    });

    let args: RemoveArgs = serde_json::from_value(json).unwrap();
    assert_eq!(args.kind, "component");
    assert_eq!(args.name, "calc");
}

/// Test RemoveArgs for profile removal
#[test]
fn test_remove_args_profile() {
    let json = serde_json::json!({
        "kind": "profile",
        "name": "dev"
    });

    let args: RemoveArgs = serde_json::from_value(json).unwrap();
    assert_eq!(args.kind, "profile");
    assert_eq!(args.name, "dev");
}

/// Test RemoveArgs missing required fields
#[test]
fn test_remove_args_missing_fields() {
    let missing_kind = serde_json::json!({"name": "test"});
    assert!(serde_json::from_value::<RemoveArgs>(missing_kind).is_err());

    let missing_name = serde_json::json!({"kind": "component"});
    assert!(serde_json::from_value::<RemoveArgs>(missing_name).is_err());
}

/// Test profile validation - output required
#[tokio::test]
async fn test_add_profile_requires_output() {
    let args = AddProfileArgs {
        name: "test".to_string(),
        components: vec!["comp1".to_string()],
        output: None,
    };

    let result = add_profile_tool(args).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err.to_string().contains("Output path is required"));
}
