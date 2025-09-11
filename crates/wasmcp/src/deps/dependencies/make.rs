use super::super::metadata::{Dependency, DependencyMetadata};

pub struct Make;

impl Dependency for Make {
    fn name(&self) -> &str {
        "make"
    }
    
    fn metadata(&self) -> DependencyMetadata {
        DependencyMetadata {
            name: self.name().to_string(),
            installation_guidance: "Usually pre-installed on Unix systems. On Linux: apt/yum install make".to_string(),
            project_url: "https://www.gnu.org/software/make/".to_string(),
            verify_command: "make --version".to_string(),
            required: false,
        }
    }
    
    fn is_installed(&self) -> bool {
        super::super::check_command_exists("make")
    }
}