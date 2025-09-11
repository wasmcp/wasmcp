use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DependencyMetadata {
    pub name: String,
    pub installation_guidance: String,
    pub project_url: String,
    pub verify_command: String,
    pub required: bool,  // Indicates if this is a required dependency
}

pub trait Dependency: Send + Sync {
    fn name(&self) -> &str;
    fn metadata(&self) -> DependencyMetadata;
    fn is_installed(&self) -> bool;
}