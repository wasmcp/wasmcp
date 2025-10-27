//! Integration tests for registry command structure
//!
//! These tests verify that the command-line interface correctly parses
//! and executes registry commands using the hierarchical structure.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Helper to create a command with a temporary config directory
fn cmd_with_temp_config() -> (Command, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("wasmcp").unwrap();

    // Set HOME to temp directory so config goes to a test location
    cmd.env("HOME", temp_dir.path());

    (cmd, temp_dir)
}

#[test]
fn test_registry_help() {
    let mut cmd = Command::cargo_bin("wasmcp").unwrap();

    cmd.args(["registry", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("component"))
        .stdout(predicate::str::contains("profile"))
        .stdout(predicate::str::contains("info"));
}

#[test]
fn test_registry_component_help() {
    let mut cmd = Command::cargo_bin("wasmcp").unwrap();

    cmd.args(["registry", "component", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("remove"))
        .stdout(predicate::str::contains("list"));
}

#[test]
fn test_registry_profile_help() {
    let mut cmd = Command::cargo_bin("wasmcp").unwrap();

    cmd.args(["registry", "profile", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("remove"))
        .stdout(predicate::str::contains("list"));
}

#[test]
fn test_registry_component_add_requires_args() {
    let mut cmd = Command::cargo_bin("wasmcp").unwrap();

    cmd.args(["registry", "component", "add"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_registry_component_add_requires_both_alias_and_spec() {
    let mut cmd = Command::cargo_bin("wasmcp").unwrap();

    cmd.args(["registry", "component", "add", "myalias"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_registry_profile_add_requires_args() {
    let mut cmd = Command::cargo_bin("wasmcp").unwrap();

    cmd.args(["registry", "profile", "add"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_registry_profile_add_requires_output_flag() {
    let mut cmd = Command::cargo_bin("wasmcp").unwrap();

    cmd.args(["registry", "profile", "add", "myprofile", "comp1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"))
        .stderr(predicate::str::contains("--output"));
}

#[test]
fn test_registry_info_with_no_config() {
    let (mut cmd, _temp_dir) = cmd_with_temp_config();

    cmd.args(["registry", "info"])
        .assert()
        .success()
        .stdout(predicate::str::contains("wasmcp Registry Information"))
        .stdout(predicate::str::contains("Config file:"))
        .stdout(predicate::str::contains("Components: 0"))
        .stdout(predicate::str::contains("Profiles:   0"))
        .stdout(predicate::str::contains("No components registered"))
        .stdout(predicate::str::contains("No profiles registered"));
}

#[test]
fn test_registry_info_components_filter() {
    let (mut cmd, _temp_dir) = cmd_with_temp_config();

    cmd.args(["registry", "info", "--components"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No components registered"))
        // Should show statistics (which includes "Profiles:   0")
        .stdout(predicate::str::contains("Statistics:"))
        .stdout(predicate::str::contains("Profiles:   0"))
        // But should NOT show the profiles list section header
        .stdout(predicate::str::contains("No profiles registered").not());
}

#[test]
fn test_registry_info_profiles_filter() {
    let (mut cmd, _temp_dir) = cmd_with_temp_config();

    cmd.args(["registry", "info", "--profiles"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Profiles:"))
        .stdout(predicate::str::contains("No profiles registered"))
        // Should still show Components: 0 in statistics but not the full components list
        .stdout(predicate::str::contains("Components: 0"));
}

#[test]
fn test_registry_info_filters_are_mutually_exclusive() {
    let mut cmd = Command::cargo_bin("wasmcp").unwrap();

    cmd.args(["registry", "info", "--components", "--profiles"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_registry_component_list_empty() {
    let (mut cmd, _temp_dir) = cmd_with_temp_config();

    cmd.args(["registry", "component", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No components registered"))
        .stdout(predicate::str::contains("registry component add"));
}

#[test]
fn test_registry_profile_list_empty() {
    let (mut cmd, _temp_dir) = cmd_with_temp_config();

    cmd.args(["registry", "profile", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No profiles registered"))
        .stdout(predicate::str::contains("registry profile add"));
}

#[test]
fn test_old_register_command_does_not_exist() {
    let mut cmd = Command::cargo_bin("wasmcp").unwrap();

    // The old flat command should not work anymore
    cmd.args(["registry", "register"]).assert().failure();
}

#[test]
fn test_old_unregister_command_does_not_exist() {
    let mut cmd = Command::cargo_bin("wasmcp").unwrap();

    // The old flat command should not work anymore
    cmd.args(["registry", "unregister"]).assert().failure();
}

#[test]
fn test_old_profile_create_command_does_not_exist() {
    let mut cmd = Command::cargo_bin("wasmcp").unwrap();

    // The old dashed command should not work anymore
    cmd.args(["registry", "profile-create"]).assert().failure();
}

#[test]
fn test_old_profile_delete_command_does_not_exist() {
    let mut cmd = Command::cargo_bin("wasmcp").unwrap();

    // The old dashed command should not work anymore
    cmd.args(["registry", "profile-delete"]).assert().failure();
}

#[test]
fn test_old_list_command_does_not_exist() {
    let mut cmd = Command::cargo_bin("wasmcp").unwrap();

    // The old separate list command should not work anymore
    cmd.args(["registry", "list"]).assert().failure();
}

#[test]
fn test_empty_state_shows_concrete_examples() {
    let (mut cmd, _temp_dir) = cmd_with_temp_config();

    cmd.args(["registry", "component", "list"])
        .assert()
        .success()
        // Check for concrete examples, not just templates
        .stdout(predicate::str::contains("wasmcp:calculator@0.1.0"))
        .stdout(predicate::str::contains(
            "./target/wasm32-wasip2/release/handler.wasm",
        ))
        .stdout(predicate::str::contains("# From a registry package:"))
        .stdout(predicate::str::contains("# From a local file:"))
        .stdout(predicate::str::contains("# From another alias:"));
}

#[test]
fn test_profile_empty_state_shows_concrete_examples() {
    let (mut cmd, _temp_dir) = cmd_with_temp_config();

    cmd.args(["registry", "profile", "list"])
        .assert()
        .success()
        // Check for concrete examples with realistic names
        .stdout(predicate::str::contains("dev-server"))
        .stdout(predicate::str::contains("prod-server"))
        .stdout(predicate::str::contains("# Simple profile:"))
        .stdout(predicate::str::contains(
            "# With inheritance from a base profile:",
        ))
        .stdout(predicate::str::contains("-b dev-server"));
}

#[test]
fn test_registry_info_short_flag_c() {
    let (mut cmd, _temp_dir) = cmd_with_temp_config();

    cmd.args(["registry", "info", "-c"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No components registered"));
}

#[test]
fn test_registry_info_short_flag_p() {
    let (mut cmd, _temp_dir) = cmd_with_temp_config();

    cmd.args(["registry", "info", "-p"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No profiles registered"));
}

#[test]
fn test_registry_info_short_flags_are_mutually_exclusive() {
    let mut cmd = Command::cargo_bin("wasmcp").unwrap();

    cmd.args(["registry", "info", "-c", "-p"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}
