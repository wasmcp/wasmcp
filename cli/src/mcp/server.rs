//! MCP server implementation
//!
//! This module contains the WasmcpServer struct and its ServerHandler implementation,
//! delegating to resource and tool modules for actual functionality.

use anyhow::Result;
use rmcp::ErrorData as McpError;
use rmcp::ServerHandler;
use rmcp::model::*;
use rmcp::service::{RequestContext, RoleServer};
use std::path::PathBuf;

/// Main MCP server struct for wasmcp
#[derive(Clone)]
pub struct WasmcpServer {
    http_client: reqwest::Client,
    project_root: PathBuf,
    local_resources_path: Option<PathBuf>,
}

impl WasmcpServer {
    /// Create a new WasmcpServer instance
    ///
    /// # Arguments
    /// * `project_root` - Current working directory (unused currently)
    /// * `local_resources_path` - Optional path to repository root for local resource override.
    ///   When set, all resource requests read from local filesystem instead of GitHub.
    pub fn new(project_root: PathBuf, local_resources_path: Option<PathBuf>) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            http_client,
            project_root,
            local_resources_path,
        })
    }
}

impl ServerHandler for WasmcpServer {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::V_2025_03_26,
            server_info: Implementation {
                name: "wasmcp-mcp-server".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: Some("wasmcp MCP Server".into()),
                icons: None,
                website_url: Some("https://github.com/wasmcp/wasmcp".into()),
            },
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            instructions: Some(
                "MCP server providing wasmcp documentation, registry management, \
                 and composition tools for AI-assisted development. \
                 IMPORTANT: All documentation and resources are available via MCP resources (wasmcp:// URIs). \
                 Use the MCP resources/read method to access content - do not attempt to read local file paths directly."
                    .into(),
            ),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let result = crate::mcp::resources::list_all(&self.project_root)?;
        tracing::info!(
            "ListResources returning {} resources",
            result.resources.len()
        );
        Ok(result)
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        // When using local resources, branch templates don't make sense
        // since we're reading from local filesystem (no branches)
        if self.local_resources_path.is_some() {
            tracing::info!("ListResourceTemplates returning 0 templates (local resources enabled)");
            return Ok(ListResourceTemplatesResult {
                resource_templates: vec![],
                next_cursor: None,
            });
        }

        let result = crate::mcp::resources::list_templates()?;
        tracing::info!(
            "ListResourceTemplates returning {} templates",
            result.resource_templates.len()
        );
        Ok(result)
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        crate::mcp::resources::read(
            &self.http_client,
            &request.uri,
            &self.project_root,
            &self.local_resources_path,
        )
        .await
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let result = crate::mcp::tools::list_tools()?;
        tracing::info!("ListTools returning {} tools", result.tools.len());
        Ok(result)
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let arguments = request.arguments.unwrap_or_default();
        crate::mcp::tools::call_tool(&request.name, arguments).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Test WasmcpServer creation without local resources
    #[test]
    fn test_server_creation_no_local_resources() {
        let temp_dir = TempDir::new().unwrap();
        let server = WasmcpServer::new(temp_dir.path().to_path_buf(), None);

        assert!(server.is_ok());
        let server = server.unwrap();
        assert_eq!(server.project_root, temp_dir.path());
        assert!(server.local_resources_path.is_none());
    }

    /// Test WasmcpServer creation with local resources path
    #[test]
    fn test_server_creation_with_local_resources() {
        let temp_dir = TempDir::new().unwrap();
        let local_path = temp_dir.path().join("repo");
        std::fs::create_dir(&local_path).unwrap();

        let server = WasmcpServer::new(temp_dir.path().to_path_buf(), Some(local_path.clone()));

        assert!(server.is_ok());
        let server = server.unwrap();
        assert!(server.local_resources_path.is_some());
        assert_eq!(server.local_resources_path.unwrap(), local_path);
    }

    /// Test get_info returns correct metadata
    #[test]
    fn test_get_info_returns_correct_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let server = WasmcpServer::new(temp_dir.path().to_path_buf(), None).unwrap();

        let info = server.get_info();

        // Verify protocol version
        assert_eq!(info.protocol_version, ProtocolVersion::V_2025_03_26);

        // Verify server info
        assert_eq!(info.server_info.name, "wasmcp-mcp-server");
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.server_info.title, Some("wasmcp MCP Server".into()));
        assert_eq!(
            info.server_info.website_url,
            Some("https://github.com/wasmcp/wasmcp".into())
        );

        // Verify capabilities
        assert!(info.capabilities.tools.is_some());
        assert!(info.capabilities.resources.is_some());

        // Verify instructions are present and mention resources
        assert!(info.instructions.is_some());
        let instructions = info.instructions.unwrap();
        assert!(instructions.contains("wasmcp://"));
        assert!(instructions.contains("resources"));
    }

    /// Test that capabilities include tools and resources
    #[test]
    fn test_server_capabilities() {
        let temp_dir = TempDir::new().unwrap();
        let server = WasmcpServer::new(temp_dir.path().to_path_buf(), None).unwrap();

        let info = server.get_info();

        // Tools should be enabled
        assert!(info.capabilities.tools.is_some());

        // Resources should be enabled
        assert!(info.capabilities.resources.is_some());

        // Prompts should not be enabled
        assert!(info.capabilities.prompts.is_none());
    }

    /// Test HTTP client is created successfully
    #[test]
    fn test_http_client_creation() {
        let temp_dir = TempDir::new().unwrap();
        let server = WasmcpServer::new(temp_dir.path().to_path_buf(), None).unwrap();

        // HTTP client should be usable (we can't test actual requests without a server)
        // But we can verify the timeout is set
        // Note: reqwest::Client doesn't expose timeout publicly, so this is a smoke test
        let _client = &server.http_client;
    }

    /// Test server is clonable
    #[test]
    fn test_server_is_clonable() {
        let temp_dir = TempDir::new().unwrap();
        let server = WasmcpServer::new(temp_dir.path().to_path_buf(), None).unwrap();

        // Should be able to clone
        let _cloned = server.clone();
    }

    /// Test local_resources_path configuration
    #[test]
    fn test_local_resources_path_configuration() {
        let temp_dir = TempDir::new().unwrap();
        let local_path = temp_dir.path().join("repo");
        std::fs::create_dir(&local_path).unwrap();

        // Server with local resources
        let server_with_local =
            WasmcpServer::new(temp_dir.path().to_path_buf(), Some(local_path.clone())).unwrap();

        assert!(server_with_local.local_resources_path.is_some());
        assert_eq!(server_with_local.local_resources_path.unwrap(), local_path);

        // Server without local resources
        let server_without_local = WasmcpServer::new(temp_dir.path().to_path_buf(), None).unwrap();

        assert!(server_without_local.local_resources_path.is_none());
    }

    /// Test instruction text contains important guidance
    #[test]
    fn test_instructions_content() {
        let temp_dir = TempDir::new().unwrap();
        let server = WasmcpServer::new(temp_dir.path().to_path_buf(), None).unwrap();

        let info = server.get_info();
        let instructions = info.instructions.unwrap();

        // Should mention MCP resources
        assert!(instructions.contains("MCP resources"));

        // Should mention the URI scheme
        assert!(instructions.contains("wasmcp://"));

        // Should warn about not reading local paths directly
        assert!(
            instructions.contains("do not attempt to read local file paths directly")
                || instructions.contains("Use the MCP resources/read method")
        );

        // Should mention documentation
        assert!(instructions.contains("documentation"));
    }

    /// Test server info version matches package version
    #[test]
    fn test_version_consistency() {
        let temp_dir = TempDir::new().unwrap();
        let server = WasmcpServer::new(temp_dir.path().to_path_buf(), None).unwrap();

        let info = server.get_info();

        // Version should match Cargo.toml
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));

        // Should be a valid semver-like string
        assert!(info.server_info.version.contains('.'));
    }
}
