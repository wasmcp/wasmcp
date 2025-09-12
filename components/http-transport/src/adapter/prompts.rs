use super::WitMcpAdapter;
use anyhow::Result;
use rmcp::model::{
    GetPromptResult, ListPromptsResult, Prompt, PromptArgument, PromptMessage,
    PromptMessageContent, PromptMessageRole,
};

impl WitMcpAdapter {
    /// Convert WIT ListPromptsResponse to rmcp ListPromptsResult
    pub fn convert_list_prompts_to_rmcp(
        &self,
        response: crate::bindings::wasmcp::mcp::prompts_types::ListPromptsResult,
    ) -> Result<ListPromptsResult> {
        let prompts = response
            .prompts
            .into_iter()
            .map(|p| {
                let arguments = p.arguments.map(|args| {
                    args.into_iter()
                        .map(|a| PromptArgument {
                            name: a.base.name,
                            description: a.description,
                            required: a.required,
                        })
                        .collect()
                });

                Prompt {
                    name: p.base.name,
                    description: p.description,
                    arguments,
                }
            })
            .collect();

        Ok(ListPromptsResult {
            prompts,
            next_cursor: response.next_cursor,
        })
    }

    /// Convert WIT GetPromptResponse to rmcp GetPromptResult
    pub fn convert_get_prompt_to_rmcp(
        &self,
        response: crate::bindings::wasmcp::mcp::prompts_types::GetPromptResult,
    ) -> Result<GetPromptResult> {
        use crate::bindings::wasmcp::mcp::mcp_types::{ContentBlock, MessageRole};

        let messages = response
            .messages
            .into_iter()
            .map(|m| {
                let role = match m.role {
                    MessageRole::User => PromptMessageRole::User,
                    MessageRole::Assistant => PromptMessageRole::Assistant,
                    MessageRole::System => PromptMessageRole::Assistant, // Map System to Assistant for rmcp compatibility
                };

                let content_text = match m.content {
                    ContentBlock::Text(t) => t.text,
                    _ => String::new(), // Skip non-text content
                };

                PromptMessage {
                    role,
                    content: PromptMessageContent::text(content_text),
                }
            })
            .collect();

        Ok(GetPromptResult {
            messages,
            description: response.description,
        })
    }
}