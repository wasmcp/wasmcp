pub mod metadata;
pub mod dependencies;

use std::process::Command;
use metadata::{Dependency, DependencyMetadata};
use dependencies::*;
use once_cell::sync::Lazy;

// Register all dependencies
static DEPENDENCIES: Lazy<Vec<Box<dyn Dependency>>> = Lazy::new(|| {
    vec![
        Box::new(wasmtime::Wasmtime),
        Box::new(spin::Spin),
        Box::new(wac::Wac),
        Box::new(wkg::Wkg),
        Box::new(make::Make),
    ]
});

/// Get metadata for a specific dependency
pub fn get_dependency_metadata(name: &str) -> Option<DependencyMetadata> {
    DEPENDENCIES.iter()
        .find(|d| d.name() == name)
        .map(|d| d.metadata())
}

// Keep existing functions from the original deps.rs (DO NOT DELETE until migration complete)

/// Check if a command exists in PATH
pub fn check_command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get dependencies required for a specific wasmcp tool
pub fn get_tool_dependencies(tool: &str) -> Vec<&'static str> {
    match tool {
        "wasmcp_list" => vec![],  // No external deps
        "wasmcp_init" => vec!["spin"],
        "wasmcp_build" => vec!["make"],  // Language-agnostic, uses Makefile
        "wasmcp_serve_spin" => vec!["spin"],
        "wasmcp_serve_wasmtime" => vec!["wasmtime"],
        "wasmcp_compose" => vec!["wac"],
        "wasmcp_validate_wit" => vec!["wkg"],
        "wasmcp_check_deps" => vec![],  // Diagnostic tool has no deps
        _ => vec![],
    }
}

/// Check if a tool has all its dependencies available
pub fn is_tool_available(tool: &str) -> bool {
    get_tool_dependencies(tool)
        .iter()
        .all(|dep| check_command_exists(dep))
}

/// Check all dependencies and return (installed, missing)
pub fn check_all_dependencies() -> (Vec<String>, Vec<String>) {
    let commands = ["make", "spin", "wasmtime", "wac", "wkg"];
    let mut installed = vec![];
    let mut missing = vec![];
    
    for cmd in commands {
        if check_command_exists(cmd) {
            installed.push(cmd.to_string());
        } else {
            missing.push(cmd.to_string());
        }
    }
    (installed, missing)
}