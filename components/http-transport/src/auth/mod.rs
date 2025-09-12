/// Authorization module for MCP transport
/// Handles JWT validation, policy enforcement, and OAuth discovery

mod authorization;
mod discovery;
pub mod http;
mod jwt;
mod policy;
mod types;

// Re-export the main authorization function
pub use authorization::authorize;

// Re-export discovery functions
pub use discovery::{get_resource_metadata, get_server_metadata};

// Re-export HTTP auth functions
pub use http::{
    authorize_request, create_auth_error_response, handle_resource_metadata,
    handle_server_metadata,
};

// Re-export types
pub use types::AuthContext;