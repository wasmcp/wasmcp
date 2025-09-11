use super::super::metadata::{Dependency, DependencyMetadata};

pub struct Wkg;

impl Dependency for Wkg {
    fn name(&self) -> &str {
        "wkg"
    }
    
    fn metadata(&self) -> DependencyMetadata {
        DependencyMetadata {
            name: self.name().to_string(),
            installation_guidance: "Install via cargo install wkg".to_string(),
            project_url: "https://github.com/bytecodealliance/wasm-pkg-tools".to_string(),
            verify_command: "wkg --version".to_string(),
            required: false,
        }
    }
    
    fn is_installed(&self) -> bool {
        super::super::check_command_exists("wkg")
    }
}