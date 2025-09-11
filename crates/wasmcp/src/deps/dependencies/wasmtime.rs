use super::super::metadata::{Dependency, DependencyMetadata};

pub struct Wasmtime;

impl Dependency for Wasmtime {
    fn name(&self) -> &str {
        "wasmtime"
    }
    
    fn metadata(&self) -> DependencyMetadata {
        DependencyMetadata {
            name: self.name().to_string(),
            installation_guidance: "Install from wasmtime.dev or via cargo install wasmtime-cli".to_string(),
            project_url: "https://wasmtime.dev".to_string(),
            verify_command: "wasmtime --version".to_string(),
            required: false,
        }
    }
    
    fn is_installed(&self) -> bool {
        super::super::check_command_exists("wasmtime")
    }
}