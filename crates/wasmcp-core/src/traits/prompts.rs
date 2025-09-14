use crate::{
    McpError,
    ListPromptsRequest, ListPromptsResult,
    GetPromptRequest, GetPromptResult
};

/// Defines the contract for MCP prompts providers.
///
/// Prompts allow servers to provide structured messages and instructions for
/// interacting with language models. Clients can discover available prompts,
/// retrieve their contents, and provide arguments to customize them.
pub trait McpPromptsHandler {
    /// List available prompts
    fn list_prompts(&self, request: ListPromptsRequest) -> Result<ListPromptsResult, McpError>;

    /// Get a specific prompt
    fn get_prompt(&self, request: GetPromptRequest) -> Result<GetPromptResult, McpError>;
}