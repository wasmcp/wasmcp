pub mod traits;
pub mod handlers;

wit_bindgen::generate!({
    world: "types",
    path: "wit",
    additional_derives: [serde::Serialize, serde::Deserialize, Clone],
    generate_unused_types: true,
});

pub use wasmcp::mcp::mcp_types::{
    ErrorCode, McpError, ContentBlock, ResourceContents, Role, Icon, Annotations, MetaFields,
    JsonValue, JsonObject, TextContent, ImageContent, AudioContent, EmbeddedResource,
    RawResource, TextResourceContents, BlobResourceContents, ProgressToken, RequestId,
    MessageRole, ModelPreferences, ModelHint
};

pub use wasmcp::mcp::lifecycle_types::{
    ProtocolVersion, Implementation, RootsCapability, PromptsCapability, ResourcesCapability,
    ToolsCapability, ElicitationCapability, ClientCapabilities, ServerCapabilities,
    InitializeRequest, InitializeResult
};

pub use wasmcp::mcp::authorization_types::{
    ProviderAuthConfig, AuthContext
};

pub use wasmcp::mcp::completion_types::{
    CompleteRequest, CompleteResult, Reference, ResourceReference, PromptReference,
    ArgumentInfo, CompletionContext, CompletionInfo
};

pub use wasmcp::mcp::prompts_types::{
    PromptArgument, Prompt, PromptMessageRole, PromptMessageContent, PromptMessage,
    ListPromptsRequest, ListPromptsResult, GetPromptRequest, GetPromptResult
};

pub use wasmcp::mcp::resources_types::{
    McpResource, ResourceTemplate, ListResourcesRequest, ListResourcesResult,
    ListResourceTemplatesRequest, ListResourceTemplatesResult, ReadResourceRequest,
    ReadResourceResult, SubscribeRequest, UnsubscribeRequest
};

pub use wasmcp::mcp::tools_types::{
    ToolAnnotations, Tool, CallToolRequest, CallToolResult, ListToolsRequest, ListToolsResult
};

pub use traits::McpLifecycleHandler;
pub use handlers::lifecycle;
