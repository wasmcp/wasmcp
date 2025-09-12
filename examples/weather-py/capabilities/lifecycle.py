"""Lifecycle implementation for weather-py MCP server."""

from wit_world.imports import lifecycle_types


class Lifecycle:
    """Handle MCP lifecycle methods."""
    
    def initialize(self, request: lifecycle_types.InitializeRequest) -> lifecycle_types.InitializeResult:
        """Initialize the MCP server."""
        return lifecycle_types.InitializeResult(
            protocol_version="0.1.0",
            capabilities=lifecycle_types.ServerCapabilities(
                experimental=None,
                logging=None,
                completions=None,
                prompts=None,
                resources=None,
                tools=lifecycle_types.ToolsCapability(list_changed=None)
            ),
            server_info=lifecycle_types.Implementation(
                name="weather-py",
                version="0.1.0",
                title="Weather Python Provider",
                icons=None,
                website_url=None
            ),
            instructions="A Python MCP server providing weather tools"
        )

    def client_initialized(self) -> None:
        """Called when client has initialized."""
        pass

    def shutdown(self) -> None:
        """Shutdown the server."""
        pass