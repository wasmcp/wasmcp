// Auto-generated bindings from cargo-component
#[allow(warnings)]
mod bindings;

// Helper modules
mod helpers;

// Result type implementations
mod list_tools_result;
mod initialize_result;
mod call_tool_result;
mod list_resources_result;
mod list_resource_templates_result;
mod read_resource_result;
mod list_prompts_result;
mod get_prompt_result;
mod complete_result;
mod context;

// Component exports
struct Component;

impl bindings::exports::wasmcp::mcp::types::Guest for Component {
    type ListToolsResult = list_tools_result::ListToolsResult;
    type InitializeResult = initialize_result::InitializeResult;
    type CallToolResult = call_tool_result::CallToolResult;
    type ListResourcesResult = list_resources_result::ListResourcesResult;
    type ListResourceTemplatesResult = list_resource_templates_result::ListResourceTemplatesResult;
    type ReadResourceResult = read_resource_result::ReadResourceResult;
    type ListPromptsResult = list_prompts_result::ListPromptsResult;
    type GetPromptResult = get_prompt_result::GetPromptResult;
    type CompleteResult = complete_result::CompleteResult;
    type Context = context::Context;
}

bindings::export!(Component with_types_in bindings);