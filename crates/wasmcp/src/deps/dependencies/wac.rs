use super::super::metadata::{Dependency, DependencyMetadata};

pub struct Wac;

impl Dependency for Wac {
    fn name(&self) -> &str {
        "wac"
    }
    
    fn metadata(&self) -> DependencyMetadata {
        DependencyMetadata {
            name: self.name().to_string(),
            installation_guidance: "Install via cargo install wac-cli".to_string(),
            project_url: "https://github.com/bytecodealliance/wac".to_string(),
            verify_command: "wac --version".to_string(),
            required: false,
        }
    }
    
    fn is_installed(&self) -> bool {
        super::super::check_command_exists("wac")
    }
}