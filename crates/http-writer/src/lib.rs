//! HTTP writer component for the Model Context Protocol (MCP).
//!
//! This component exports all writer interfaces for serializing MCP messages
//! to output streams with Server-Sent Events (SSE) framing for HTTP transport.
//!
//! SSE Format:
//! - Each message is prefixed with "data: "
//! - Messages end with double newline "\n\n"
//! - Multi-line JSON is split with "data: " on each line
//!
//! This implementation follows the MCP 2025-06-18 specification for
//! Streamable HTTP transport using text/event-stream responses.

mod bindings {
    wit_bindgen::generate!({
        world: "http-writer",
        generate_all,
    });
}

// Shared utilities for all writers
mod utils;

// Individual writer implementations
mod notifications;
mod error;
mod empty;
mod initialize;
mod tools;
mod resources;
mod prompts;
mod completion;

struct Component;

// Implement each Guest trait by delegating to the module implementations
impl bindings::exports::wasmcp::mcp::notifications_writer::Guest for Component {
    fn log(out: &bindings::wasi::io::streams::OutputStream, message: bindings::wasmcp::mcp::protocol::LogMessage) -> Result<(), bindings::wasi::io::streams::StreamError> {
        notifications::NotificationsWriter::log(out, message)
    }

    fn send(out: &bindings::wasi::io::streams::OutputStream, notification: bindings::wasmcp::mcp::protocol::ClientNotification) -> Result<(), bindings::wasi::io::streams::StreamError> {
        notifications::NotificationsWriter::send(out, notification)
    }

    fn send_list_changed(out: &bindings::wasi::io::streams::OutputStream, change: bindings::wasmcp::mcp::protocol::ChangeNotificationType) -> Result<(), bindings::wasi::io::streams::StreamError> {
        notifications::NotificationsWriter::send_list_changed(out, change)
    }

    fn send_updated(out: &bindings::wasi::io::streams::OutputStream, update: bindings::wasmcp::mcp::protocol::UpdateNotificationType) -> Result<(), bindings::wasi::io::streams::StreamError> {
        notifications::NotificationsWriter::send_updated(out, update)
    }

    fn send_progress(out: &bindings::wasi::io::streams::OutputStream, progress_token: bindings::wasmcp::mcp::protocol::ProgressToken, progress: f64, total: Option<f64>, message: Option<String>) -> Result<(), bindings::wasi::io::streams::StreamError> {
        notifications::NotificationsWriter::send_progress(out, progress_token, progress, total, message)
    }
}

impl bindings::exports::wasmcp::mcp::error_writer::Guest for Component {
    fn send(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream, error: bindings::wasmcp::mcp::protocol::Error) -> Result<(), bindings::wasi::io::streams::StreamError> {
        error::ErrorWriter::send(id, out, error)
    }
}

impl bindings::exports::wasmcp::mcp::empty_writer::Guest for Component {
    fn send(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream) -> Result<(), bindings::wasi::io::streams::StreamError> {
        empty::EmptyWriter::send(id, out)
    }
}

impl bindings::exports::wasmcp::mcp::initialize_writer::Guest for Component {
    fn send(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream, result: bindings::exports::wasmcp::mcp::initialize_writer::InitializeResult) -> Result<(), bindings::wasi::io::streams::StreamError> {
        initialize::InitializeWriter::send(id, out, result)
    }
}

// Stub implementations for now - will be completed properly
impl bindings::exports::wasmcp::mcp::list_tools_writer::Guest for Component {
    fn send(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream, tools: Vec<bindings::exports::wasmcp::mcp::list_tools_writer::Tool>) -> Result<(), bindings::wasi::io::streams::StreamError> {
        tools::ListToolsWriter::send(id, out, tools)
    }

    type Writer = tools::ListToolsWriterResource;

    fn open(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream) -> Result<bindings::exports::wasmcp::mcp::list_tools_writer::Writer, bindings::wasi::io::streams::StreamError> {
        tools::ListToolsWriter::open(id, out)
    }
}

impl bindings::exports::wasmcp::mcp::content_tool_writer::Guest for Component {
    fn send_text(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream, text: String) -> Result<(), bindings::wasi::io::streams::StreamError> {
        tools::ContentToolWriter::send_text(id, out, text)
    }

    fn send_error(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream, reason: String) -> Result<(), bindings::wasi::io::streams::StreamError> {
        tools::ContentToolWriter::send_error(id, out, reason)
    }

    fn send_contents(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream, content: Vec<bindings::wasmcp::mcp::protocol::ContentBlock>) -> Result<(), bindings::wasi::io::streams::StreamError> {
        tools::ContentToolWriter::send_contents(id, out, content)
    }

    type Writer = tools::ContentToolWriterResource;

    fn open(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream, initial: bindings::wasmcp::mcp::protocol::ContentBlock) -> Result<bindings::exports::wasmcp::mcp::content_tool_writer::Writer, bindings::wasi::io::streams::StreamError> {
        tools::ContentToolWriter::open(id, out, initial)
    }
}

impl bindings::exports::wasmcp::mcp::structured_tool_writer::Guest for Component {
    fn send(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream, structured: bindings::wasmcp::mcp::protocol::StructuredToolResult, options: Option<bindings::wasmcp::mcp::protocol::IsErrorOptions>) -> Result<(), bindings::wasi::io::streams::StreamError> {
        tools::StructuredToolWriter::send(id, out, structured, options)
    }
}

// Continue with remaining stub implementations...
impl bindings::exports::wasmcp::mcp::list_resources_writer::Guest for Component {
    fn send(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream, resources: Vec<bindings::exports::wasmcp::mcp::list_resources_writer::Resource>) -> Result<(), bindings::wasi::io::streams::StreamError> {
        resources::ListResourcesWriter::send(id, out, resources)
    }

    type Writer = resources::ListResourcesWriterResource;

    fn open(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream) -> Result<bindings::exports::wasmcp::mcp::list_resources_writer::Writer, bindings::wasi::io::streams::StreamError> {
        resources::ListResourcesWriter::open(id, out)
    }
}

impl bindings::exports::wasmcp::mcp::read_resource_writer::Guest for Component {
    fn send(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream, contents: bindings::exports::wasmcp::mcp::read_resource_writer::ResourceContents) -> Result<(), bindings::wasi::io::streams::StreamError> {
        resources::ReadResourceWriter::send(id, out, contents)
    }

    type Writer = resources::ReadResourceWriterResource;

    fn open(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream, initial: bindings::exports::wasmcp::mcp::read_resource_writer::ResourceContents) -> Result<bindings::exports::wasmcp::mcp::read_resource_writer::Writer, bindings::wasi::io::streams::StreamError> {
        resources::ReadResourceWriter::open(id, out, initial)
    }
}

impl bindings::exports::wasmcp::mcp::list_resource_templates_writer::Guest for Component {
    fn send(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream, templates: Vec<bindings::exports::wasmcp::mcp::list_resource_templates_writer::ResourceTemplate>) -> Result<(), bindings::wasi::io::streams::StreamError> {
        resources::ListResourceTemplatesWriter::send(id, out, templates)
    }

    type Writer = resources::ListResourceTemplatesWriterResource;

    fn open(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream) -> Result<bindings::exports::wasmcp::mcp::list_resource_templates_writer::Writer, bindings::wasi::io::streams::StreamError> {
        resources::ListResourceTemplatesWriter::open(id, out)
    }
}

impl bindings::exports::wasmcp::mcp::list_prompts_writer::Guest for Component {
    fn send(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream, prompts: Vec<bindings::exports::wasmcp::mcp::list_prompts_writer::Prompt>) -> Result<(), bindings::wasi::io::streams::StreamError> {
        prompts::ListPromptsWriter::send(id, out, prompts)
    }

    type Writer = prompts::ListPromptsWriterResource;

    fn open(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream) -> Result<bindings::exports::wasmcp::mcp::list_prompts_writer::Writer, bindings::wasi::io::streams::StreamError> {
        prompts::ListPromptsWriter::open(id, out)
    }
}

impl bindings::exports::wasmcp::mcp::get_prompt_writer::Guest for Component {
    fn send(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream, messages: Vec<bindings::exports::wasmcp::mcp::get_prompt_writer::PromptMessage>) -> Result<(), bindings::wasi::io::streams::StreamError> {
        prompts::GetPromptWriter::send(id, out, messages)
    }

    type Writer = prompts::GetPromptWriterResource;

    fn open(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream) -> Result<bindings::exports::wasmcp::mcp::get_prompt_writer::Writer, bindings::wasi::io::streams::StreamError> {
        prompts::GetPromptWriter::open(id, out)
    }
}

impl bindings::exports::wasmcp::mcp::complete_writer::Guest for Component {
    fn send(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream, values: Vec<String>, options: Option<bindings::wasmcp::mcp::protocol::CompletionResultOptions>) -> Result<(), bindings::wasi::io::streams::StreamError> {
        completion::CompleteWriter::send(id, out, values, options)
    }

    type Writer = completion::CompleteWriterResource;

    fn open(id: bindings::wasmcp::mcp::protocol::Id, out: bindings::wasi::io::streams::OutputStream) -> Result<bindings::exports::wasmcp::mcp::complete_writer::Writer, bindings::wasi::io::streams::StreamError> {
        completion::CompleteWriter::open(id, out)
    }
}

bindings::export!(Component with_types_in bindings);