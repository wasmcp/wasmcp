//! Component import validation for WebAssembly composition
//!
//! This module tracks and validates unsatisfied imports during composition
//! to ensure all component dependencies are properly wired.

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

use super::introspection::check_component_imports;

// TODO: Complete validation implementation per .agent/wire-troubleshooting.md
// These structures and methods are scaffolded but not fully wired up yet.
// Remove #[allow(dead_code)] when implementing the full validation.

/// Tracks unsatisfied imports for validation
#[derive(Debug)]
pub struct UnsatisfiedImports {
    /// Map of component name -> list of unsatisfied import interfaces
    pub imports: HashMap<String, Vec<String>>,
}

#[allow(dead_code)]
impl Default for UnsatisfiedImports {
    fn default() -> Self {
        Self::new()
    }
}

impl UnsatisfiedImports {
    pub fn new() -> Self {
        Self {
            imports: HashMap::new(),
        }
    }

    /// Add a component's imports (excluding WASI imports)
    pub fn add_component_imports(&mut self, name: String, component_path: &Path) -> Result<()> {
        let imports = check_component_imports(component_path)?;
        let non_wasi: Vec<String> = imports
            .into_iter()
            .filter(|import| !import.starts_with("wasi:"))
            .collect();

        if !non_wasi.is_empty() {
            self.imports.insert(name, non_wasi);
        }
        Ok(())
    }

    /// Mark an import as satisfied for a specific component
    pub fn mark_satisfied(&mut self, component_name: &str, interface: &str) {
        if let Some(imports) = self.imports.get_mut(component_name) {
            imports.retain(|i| i != interface);
            if imports.is_empty() {
                self.imports.remove(component_name);
            }
        }
    }

    /// Check if any imports remain unsatisfied
    pub fn has_unsatisfied(&self) -> bool {
        !self.imports.is_empty()
    }

    /// Get formatted error message for remaining unsatisfied imports
    pub fn error_message(&self) -> String {
        let mut msg = String::from("Composition has unsatisfied imports:\n");
        for (component, imports) in &self.imports {
            msg.push_str(&format!("  Component '{}':\n", component));
            for import in imports {
                msg.push_str(&format!("    - {}\n", import));
            }
        }
        msg.push_str("\nThese imports were not wired during composition. ");
        msg.push_str(
            "Check that you're wiring all required framework interfaces to user components.",
        );
        msg
    }
}
