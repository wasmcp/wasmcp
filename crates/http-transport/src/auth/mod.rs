/// Authorization module for MCP transport
/// Handles JWT validation, policy enforcement, and OAuth discovery

mod authorization;
mod http;
mod jwt;
mod policy;
mod types;

// Re-export the main authorization function
pub use authorization::authorize;
pub use http::authorize_request;
pub use types::AuthContext;
