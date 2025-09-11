// Auth module - provides JWT validation, policy engine, and OAuth discovery
// These will be implemented generically over the runtime traits

pub mod jwt;
pub mod policy;
pub mod discovery;

pub use jwt::JwtValidator;
pub use policy::PolicyEngine;
pub use discovery::OAuthDiscovery;