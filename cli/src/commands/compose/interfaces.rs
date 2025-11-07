//! Centralized interface and component naming for wasmcp composition
//!
//! This module provides the single source of truth for all interface and component
//! naming in the wasmcp system. It eliminates hardcoded strings throughout the codebase
//! by providing enums and methods that construct names dynamically from the versions.toml file.
//!
//! # Design Principles
//!
//! - Only "wasmcp:mcp-" prefix is hardcoded
//! - MCP spec version (DEFAULT_SPEC_VERSION) is a constant
//! - Component versions come from VersionResolver (versions.toml)
//! - Interface and component types are enums
//! - All name construction is centralized in this module

use crate::versioning::VersionResolver;
use anyhow::Result;

/// MCP protocol spec version
pub const DEFAULT_SPEC_VERSION: &str = "mcp-v20250618";

/// Namespace prefix for all wasmcp components and interfaces
pub const WASMCP_NAMESPACE: &str = "wasmcp";

/// WASI HTTP incoming handler interface
pub const WASI_HTTP_HANDLER: &str = "wasi:http/incoming-handler@0.2.6";

/// WASI CLI run interface
pub const WASI_CLI_RUN: &str = "wasi:cli/run@0.2.6";

/// MCP capability and framework interface types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceType {
    /// server-handler interface
    ServerHandler,
    /// tools interface
    Tools,
    /// resources interface
    Resources,
    /// prompts interface
    Prompts,
    /// server-io interface
    ServerIo,
    /// sessions interface
    Sessions,
    /// session-manager interface
    SessionManager,
}

impl InterfaceType {
    /// Get the interface name (without namespace/version)
    pub fn name(&self) -> &'static str {
        match self {
            Self::ServerHandler => "server-handler",
            Self::Tools => "tools",
            Self::Resources => "resources",
            Self::Prompts => "prompts",
            Self::ServerIo => "server-io",
            Self::Sessions => "sessions",
            Self::SessionManager => "session-manager",
        }
    }

    /// Build the full interface name with version
    ///
    /// Format: wasmcp:mcp-v20250618/interface-name@version
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmcp::commands::compose::interfaces::InterfaceType;
    /// let interface = InterfaceType::ServerHandler.interface_name("mcp-v20250618", "0.1.4");
    /// assert_eq!(interface, "wasmcp:mcp-v20250618/server-handler@0.1.4");
    /// ```
    pub fn interface_name(&self, spec_version: &str, version: &str) -> String {
        format!(
            "{}:{}/{}@{}",
            WASMCP_NAMESPACE,
            spec_version,
            self.name(),
            version
        )
    }

    /// Build an interface name prefix for matching (without version)
    ///
    /// Format: wasmcp:mcp-v20250618/interface-name@
    ///
    /// This is used for finding interfaces in component exports/imports when the
    /// exact version is unknown and needs to be discovered.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmcp::commands::compose::interfaces::InterfaceType;
    /// let prefix = InterfaceType::ServerHandler.interface_prefix("mcp-v20250618");
    /// assert_eq!(prefix, "wasmcp:mcp-v20250618/server-handler@");
    /// ```
    pub fn interface_prefix(&self, spec_version: &str) -> String {
        format!("{}:{}/{}@", WASMCP_NAMESPACE, spec_version, self.name())
    }
}

/// All MCP capability interfaces (tools, resources, prompts)
pub const CAPABILITY_INTERFACES: &[InterfaceType] = &[
    InterfaceType::Tools,
    InterfaceType::Resources,
    InterfaceType::Prompts,
];

/// Framework component types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    /// HTTP transport component
    HttpTransport,
    /// Stdio transport component
    StdioTransport,
    /// Method-not-found terminal handler
    MethodNotFound,
    /// Tools capability middleware
    ToolsMiddleware,
    /// Resources capability middleware
    ResourcesMiddleware,
    /// Prompts capability middleware
    PromptsMiddleware,
    /// Server I/O component
    ServerIo,
    /// Session store component
    SessionStore,
}

impl ComponentType {
    /// Get the component name (used in package specs and filenames)
    pub fn name(&self) -> &'static str {
        match self {
            // Both http and stdio use the same "transport" package
            Self::HttpTransport => "transport",
            Self::StdioTransport => "transport",
            Self::MethodNotFound => "method-not-found",
            Self::ToolsMiddleware => "tools-middleware",
            Self::ResourcesMiddleware => "resources-middleware",
            Self::PromptsMiddleware => "prompts-middleware",
            Self::ServerIo => "server-io",
            Self::SessionStore => "session-store",
        }
    }

    /// Build a package spec for this component
    ///
    /// Format: wasmcp:component-name@version
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmcp::commands::compose::interfaces::ComponentType;
    /// let spec = ComponentType::HttpTransport.package_spec("0.1.4");
    /// assert_eq!(spec, "wasmcp:transport@0.1.4");
    /// ```
    pub fn package_spec(&self, version: &str) -> String {
        format!("{}:{}@{}", WASMCP_NAMESPACE, self.name(), version)
    }

    /// Build a filename for this component
    ///
    /// Format: wasmcp_component-name@version.wasm
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmcp::commands::compose::interfaces::ComponentType;
    /// let filename = ComponentType::HttpTransport.filename("0.1.4");
    /// assert_eq!(filename, "wasmcp_transport@0.1.4.wasm");
    /// ```
    pub fn filename(&self, version: &str) -> String {
        format!("wasmcp_{}@{}.wasm", self.name(), version)
    }

    /// Resolve package spec using VersionResolver
    ///
    /// Looks up the version for this component from versions.toml and constructs
    /// the full package spec.
    ///
    /// # Errors
    ///
    /// Returns an error if the component version is not found in versions.toml.
    pub fn resolve_package_spec(&self, resolver: &VersionResolver) -> Result<String> {
        let version = resolver.get_version(self.name())?;
        Ok(self.package_spec(&version))
    }

    /// Resolve filename using VersionResolver
    ///
    /// Looks up the version for this component from versions.toml and constructs
    /// the full filename.
    ///
    /// # Errors
    ///
    /// Returns an error if the component version is not found in versions.toml.
    pub fn resolve_filename(&self, resolver: &VersionResolver) -> Result<String> {
        let version = resolver.get_version(self.name())?;
        Ok(self.filename(&version))
    }
}

/// Build a complete interface name for server-handler
///
/// This is a convenience function for the most commonly used interface.
///
/// # Examples
///
/// ```
/// # use wasmcp::commands::compose::interfaces::server_handler;
/// let interface = server_handler("0.1.4");
/// assert_eq!(interface, "wasmcp:mcp-v20250618/server-handler@0.1.4");
/// ```
pub fn server_handler(version: &str) -> String {
    InterfaceType::ServerHandler.interface_name(DEFAULT_SPEC_VERSION, version)
}

/// Build a complete interface name for tools
pub fn tools(version: &str) -> String {
    InterfaceType::Tools.interface_name(DEFAULT_SPEC_VERSION, version)
}

/// Build a complete interface name for resources
pub fn resources(version: &str) -> String {
    InterfaceType::Resources.interface_name(DEFAULT_SPEC_VERSION, version)
}

/// Build a complete interface name for prompts
pub fn prompts(version: &str) -> String {
    InterfaceType::Prompts.interface_name(DEFAULT_SPEC_VERSION, version)
}

/// Build a package spec
///
/// Format: wasmcp:package-name@version
pub fn package(name: &str, version: &str) -> String {
    format!("{}:{}@{}", WASMCP_NAMESPACE, name, version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_type_names() {
        assert_eq!(InterfaceType::ServerHandler.name(), "server-handler");
        assert_eq!(InterfaceType::Tools.name(), "tools");
        assert_eq!(InterfaceType::Resources.name(), "resources");
        assert_eq!(InterfaceType::Prompts.name(), "prompts");
        assert_eq!(InterfaceType::ServerIo.name(), "server-io");
        assert_eq!(InterfaceType::Sessions.name(), "sessions");
        assert_eq!(InterfaceType::SessionManager.name(), "session-manager");
    }

    #[test]
    fn test_interface_name_construction() {
        let interface = InterfaceType::ServerHandler.interface_name("mcp-v20250618", "0.1.4");
        assert_eq!(interface, "wasmcp:mcp-v20250618/server-handler@0.1.4");

        let tools_interface = InterfaceType::Tools.interface_name("mcp-v20250618", "0.1.4");
        assert_eq!(tools_interface, "wasmcp:mcp-v20250618/tools@0.1.4");
    }

    #[test]
    fn test_interface_prefix() {
        let prefix = InterfaceType::ServerHandler.interface_prefix("mcp-v20250618");
        assert_eq!(prefix, "wasmcp:mcp-v20250618/server-handler@");

        let tools_prefix = InterfaceType::Tools.interface_prefix("mcp-v20250618");
        assert_eq!(tools_prefix, "wasmcp:mcp-v20250618/tools@");
    }

    #[test]
    fn test_component_type_names() {
        // Both transport types use the same "transport" package
        assert_eq!(ComponentType::HttpTransport.name(), "transport");
        assert_eq!(ComponentType::StdioTransport.name(), "transport");
        assert_eq!(ComponentType::MethodNotFound.name(), "method-not-found");
        assert_eq!(ComponentType::ToolsMiddleware.name(), "tools-middleware");
        assert_eq!(
            ComponentType::ResourcesMiddleware.name(),
            "resources-middleware"
        );
        assert_eq!(
            ComponentType::PromptsMiddleware.name(),
            "prompts-middleware"
        );
        assert_eq!(ComponentType::ServerIo.name(), "server-io");
        assert_eq!(ComponentType::SessionStore.name(), "session-store");
    }

    #[test]
    fn test_component_package_spec() {
        let spec = ComponentType::HttpTransport.package_spec("0.1.4");
        assert_eq!(spec, "wasmcp:transport@0.1.4");

        let mnf_spec = ComponentType::MethodNotFound.package_spec("0.1.4");
        assert_eq!(mnf_spec, "wasmcp:method-not-found@0.1.4");
    }

    #[test]
    fn test_component_filename() {
        let filename = ComponentType::HttpTransport.filename("0.1.4");
        assert_eq!(filename, "wasmcp_transport@0.1.4.wasm");

        let mnf_filename = ComponentType::MethodNotFound.filename("0.1.4");
        assert_eq!(mnf_filename, "wasmcp_method-not-found@0.1.4.wasm");
    }

    #[test]
    fn test_capability_interfaces_array() {
        assert_eq!(CAPABILITY_INTERFACES.len(), 3);
        assert!(CAPABILITY_INTERFACES.contains(&InterfaceType::Tools));
        assert!(CAPABILITY_INTERFACES.contains(&InterfaceType::Resources));
        assert!(CAPABILITY_INTERFACES.contains(&InterfaceType::Prompts));
    }

    #[test]
    fn test_server_handler_convenience() {
        let interface = server_handler("0.1.4");
        assert_eq!(interface, "wasmcp:mcp-v20250618/server-handler@0.1.4");
    }

    #[test]
    fn test_tools_convenience() {
        let interface = tools("0.1.4");
        assert_eq!(interface, "wasmcp:mcp-v20250618/tools@0.1.4");
    }

    #[test]
    fn test_package_convenience() {
        let spec = package("transport", "0.1.4");
        assert_eq!(spec, "wasmcp:transport@0.1.4");
    }

    #[test]
    fn test_wasi_constants() {
        assert_eq!(WASI_HTTP_HANDLER, "wasi:http/incoming-handler@0.2.6");
        assert_eq!(WASI_CLI_RUN, "wasi:cli/run@0.2.6");
    }

    #[test]
    fn test_version_resolution_integration() {
        let resolver = VersionResolver::new().unwrap();

        // Test that all component types can resolve their versions
        assert!(
            ComponentType::HttpTransport
                .resolve_package_spec(&resolver)
                .is_ok()
        );
        assert!(
            ComponentType::MethodNotFound
                .resolve_package_spec(&resolver)
                .is_ok()
        );
        assert!(
            ComponentType::ToolsMiddleware
                .resolve_package_spec(&resolver)
                .is_ok()
        );
    }

    #[test]
    fn test_interface_type_iteration() {
        // Verify we can iterate over capability interfaces
        let mut count = 0;
        for interface in CAPABILITY_INTERFACES {
            assert!(matches!(
                interface,
                InterfaceType::Tools | InterfaceType::Resources | InterfaceType::Prompts
            ));
            count += 1;
        }
        assert_eq!(count, 3);
    }

    #[test]
    fn test_component_to_interface_mapping() {
        // Verify tools middleware maps to tools interface
        assert_eq!(ComponentType::ToolsMiddleware.name(), "tools-middleware");
        assert_eq!(InterfaceType::Tools.name(), "tools");

        // Verify resources middleware maps to resources interface
        assert_eq!(
            ComponentType::ResourcesMiddleware.name(),
            "resources-middleware"
        );
        assert_eq!(InterfaceType::Resources.name(), "resources");

        // Verify prompts middleware maps to prompts interface
        assert_eq!(
            ComponentType::PromptsMiddleware.name(),
            "prompts-middleware"
        );
        assert_eq!(InterfaceType::Prompts.name(), "prompts");
    }
}
