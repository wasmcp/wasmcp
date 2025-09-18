"""Lifecycle implementation for weather-py MCP server."""

from wit_world import exports
from wit_world.imports.wasmcp_mcp_types import Context, ServerInfo, ServerCapabilities


class Lifecycle(exports.Lifecycle):
    """Handle MCP lifecycle methods."""
    
    def initialize(self, ctx: Context) -> ServerInfo:
        """Initialize the MCP server.
        
        Returns server capabilities and metadata. The returned object is a
        dataclass generated from the WIT record type. None values map to
        WIT's option<T> when absent.
        """
        return ServerInfo("weather-py", "0.1.0", [ServerCapabilities.TOOLS])
