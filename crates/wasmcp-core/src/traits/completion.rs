use crate::{
    McpError,
    CompleteRequest, CompleteResult
};

/// Defines the contract for MCP completion providers.
///
/// Provides a standardized way for servers to offer argument autocompletion
/// suggestions for prompts and resource URIs.
pub trait McpCompletionHandler {
    /// Handle request for completion suggestions
    fn complete(&self, request: CompleteRequest) -> Result<CompleteResult, McpError>;
}