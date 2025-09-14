use crate::{
    McpError,
    ListResourcesRequest, ListResourcesResult,
    ReadResourceRequest, ReadResourceResult
};

/// Defines the contract for MCP resources providers.
///
/// Resources allow servers to share data that provides context to language models,
/// such as files, database schemas, or application-specific information.
/// Each resource is uniquely identified by a URI.
pub trait McpResourcesHandler {
    /// List available resources
    fn list_resources(&self, request: ListResourcesRequest) -> Result<ListResourcesResult, McpError>;

    /// Read a specific resource
    fn read_resource(&self, request: ReadResourceRequest) -> Result<ReadResourceResult, McpError>;
}