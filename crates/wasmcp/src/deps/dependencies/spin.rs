use super::super::metadata::{Dependency, DependencyMetadata};

pub struct Spin;

impl Dependency for Spin {
    fn name(&self) -> &str {
        "spin"
    }
    
    fn metadata(&self) -> DependencyMetadata {
        DependencyMetadata {
            name: self.name().to_string(),
            installation_guidance: "Install via npm, cargo, or Fermyon installer".to_string(),
            project_url: "https://developer.fermyon.com/spin".to_string(),
            verify_command: "spin --version".to_string(),
            required: false,
        }
    }
    
    fn is_installed(&self) -> bool {
        super::super::check_command_exists("spin")
    }
}