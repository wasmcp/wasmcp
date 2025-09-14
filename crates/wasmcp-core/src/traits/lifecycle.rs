use rmcp::model::{InitializeRequestParam, InitializeResult};

// Import the WIT-generated error type
use crate::McpError;

/// Defines the contract for any MCP lifecycle provider.
///
/// This trait uses the strongly-typed `rmcp::model` structs for request/response types
/// and the WIT-generated `McpError` type for consistent error handling.
pub trait McpLifecycleHandler {
    /// Handle the initialize request from a client
    fn initialize(&self, params: InitializeRequestParam) -> Result<InitializeResult, McpError>;

    /// Handle the client_initialized notification
    fn client_initialized(&self) -> Result<(), McpError>;

    /// Handle the shutdown request
    fn shutdown(&self) -> Result<(), McpError>;
}