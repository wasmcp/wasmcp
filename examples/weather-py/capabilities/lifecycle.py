"""Lifecycle implementation for weather-py MCP server."""

from wit_world.imports import lifecycle_types


class Lifecycle:
    """Handle MCP lifecycle methods.
    
    This class is instantiated by componentize-py and its methods are called
    directly by the WebAssembly runtime. Unlike Go, Python doesn't need special
    Result types - componentize-py handles the WIT result<T, E> mapping transparently.
    """
    
    def initialize(self, request: lifecycle_types.InitializeRequest) -> lifecycle_types.InitializeResult:
        """Initialize the MCP server.
        
        Returns server capabilities and metadata. The returned object is a
        dataclass generated from the WIT record type. None values map to
        WIT's option<T> when absent.
        """
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
        """Called when client has initialized.
        
        Note: Unlike traditional Python servers, there's no event loop or
        async context here. Each method call is synchronous and stateless -
        the Component Model handles all the async transport details.
        """
        pass

    def shutdown(self) -> None:
        """Shutdown the server.
        
        In the Component Model, the runtime manages the component lifecycle.
        This method allows for graceful cleanup if needed.
        """
        pass