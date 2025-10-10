//! Lifecycle Component for WebAssembly Component Model Context Protocol (wasmcp)
//!
//! This component provides core lifecycle management and response writing capabilities
//! for MCP servers. It serves two key roles:
//!
//! ## 1. Lifecycle Handler
//!
//! Acts as the terminal handler in a middleware chain, processing:
//! - `initialize` - Establishes MCP connection and negotiates capabilities
//! - `ping` - Health check and keep-alive
//! - Unknown methods - Returns MethodNotFound errors
//!
//! ## 2. Response Writer Library
//!
//! Exports all MCP response serialization interfaces used by feature handlers:
//! - Error responses
//! - Lifecycle responses (initialization, ping)
//! - Notifications (logging, progress, list changes, updates)
//! - Tools responses (listings, call results, streaming content)
//! - Resources responses (listings, content, templates)
//! - Prompts responses (listings, messages)
//! - Completions responses (auto-complete suggestions)
//!
//! ## Architecture
//!
//! The component uses the new stateful output interface where message writing
//! follows a start/write/finish state machine enforced at the interface level.
//! This ensures proper SSE framing and prevents message interleaving.

mod bindings {
    wit_bindgen::generate!({
        world: "lifecycle",
        generate_all,
    });
}

// Module structure
mod handler;
mod responses;
mod utils;

struct Component;

// ===== Message Handler Implementation =====

impl bindings::exports::wasmcp::mcp::message_handler::Guest for Component {
    fn handle(msg: bindings::wasmcp::mcp::protocol::McpMessage) {
        // Delegate to the handler module
        // Errors are handled by writing error responses, not propagated
        let _ = handler::handle_message(&msg);
    }
}

// ===== Notification Response Implementation =====

impl bindings::exports::wasmcp::mcp::notification_response::Guest for Component {
    fn log(
        message: bindings::wasmcp::mcp::protocol::LogMessage,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::notifications::log(message)
    }

    fn notify(
        notification: bindings::wasmcp::mcp::protocol::ClientNotification,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::notifications::notify(notification)
    }

    fn notify_list_changed(
        change: bindings::wasmcp::mcp::protocol::ChangeNotificationType,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::notifications::notify_list_changed(change)
    }

    fn notify_updated(
        update: bindings::wasmcp::mcp::protocol::UpdateNotificationType,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::notifications::notify_updated(update)
    }

    fn notify_progress(
        progress_token: bindings::wasmcp::mcp::protocol::ProgressToken,
        progress: f64,
        total: Option<f64>,
        message: Option<String>,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::notifications::notify_progress(progress_token, progress, total, message)
    }
}

// ===== Error Response Implementation =====

impl bindings::exports::wasmcp::mcp::error_response::Guest for Component {
    fn write(
        error: bindings::wasmcp::mcp::protocol::McpError,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::error::write_error(error)
    }
}

// ===== Lifecycle Response Implementation =====

impl bindings::exports::wasmcp::mcp::lifecycle_response::Guest for Component {
    fn write_initialization(
        id: bindings::wasmcp::mcp::protocol::Id,
        result: bindings::wasmcp::mcp::protocol::InitializeResult,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::lifecycle::write_initialization(id, result)
    }

    fn write_pong(
        id: bindings::wasmcp::mcp::protocol::Id,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::lifecycle::write_pong(id)
    }
}

// ===== Tools Response Implementation =====

impl bindings::exports::wasmcp::mcp::tools_response::Guest for Component {
    fn write_tools(
        id: bindings::wasmcp::mcp::protocol::Id,
        tools: Vec<bindings::wasmcp::mcp::protocol::Tool>,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::tools::write_tools(id, tools)
    }

    fn write_text(
        id: bindings::wasmcp::mcp::protocol::Id,
        text: String,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::tools::write_text(id, text)
    }

    fn write_error(
        id: bindings::wasmcp::mcp::protocol::Id,
        reason: String,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::tools::write_error(id, reason)
    }

    fn write_content_blocks(
        id: bindings::wasmcp::mcp::protocol::Id,
        content: Vec<bindings::wasmcp::mcp::protocol::ContentBlock>,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::tools::write_content_blocks(id, content)
    }

    fn write_structured(
        id: bindings::wasmcp::mcp::protocol::Id,
        structured: bindings::wasmcp::mcp::protocol::StructuredToolResult,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::tools::write_structured(id, structured)
    }

    type ToolsWriter = responses::tools::ToolsWriter;
    type ContentBlocksWriter = responses::tools::ContentBlocksWriter;
}

// ===== Resources Response Implementation =====

impl bindings::exports::wasmcp::mcp::resources_response::Guest for Component {
    fn write_resources(
        id: bindings::wasmcp::mcp::protocol::Id,
        resources: Vec<bindings::wasmcp::mcp::protocol::Resource>,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::resources::write_resources(id, resources)
    }

    fn write_contents(
        id: bindings::wasmcp::mcp::protocol::Id,
        contents: bindings::wasmcp::mcp::protocol::ResourceContents,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::resources::write_contents(id, contents)
    }

    fn write_templates(
        id: bindings::wasmcp::mcp::protocol::Id,
        templates: Vec<bindings::wasmcp::mcp::protocol::ResourceTemplate>,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::resources::write_templates(id, templates)
    }

    type ResourcesWriter = responses::resources::ResourcesWriter;
    type ContentsWriter = responses::resources::ContentsWriter;
    type TemplatesWriter = responses::resources::TemplatesWriter;
}

// ===== Prompts Response Implementation =====

impl bindings::exports::wasmcp::mcp::prompts_response::Guest for Component {
    fn write_prompts(
        id: bindings::wasmcp::mcp::protocol::Id,
        prompts: Vec<bindings::wasmcp::mcp::protocol::Prompt>,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::prompts::write_prompts(id, prompts)
    }

    fn write_prompt_messages(
        id: bindings::wasmcp::mcp::protocol::Id,
        messages: Vec<bindings::wasmcp::mcp::protocol::PromptMessage>,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::prompts::write_prompt_messages(id, messages)
    }

    type PromptsWriter = responses::prompts::PromptsWriter;
    type PromptMessagesWriter = responses::prompts::PromptMessagesWriter;
}

// ===== Completions Response Implementation =====

impl bindings::exports::wasmcp::mcp::completions_response::Guest for Component {
    fn write_completions(
        id: bindings::wasmcp::mcp::protocol::Id,
        completions: bindings::exports::wasmcp::mcp::completions_response::Completions,
    ) -> Result<(), bindings::wasmcp::mcp::output::IoError> {
        responses::completions::write_completions(id, completions)
    }

    type CompletionsWriter = responses::completions::CompletionsWriter;
}

// Export the component
bindings::export!(Component with_types_in bindings);
