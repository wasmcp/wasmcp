"""wasmcp Python SDK - WebAssembly MCP handler framework."""

from .handler import WasmcpHandler, Handler
from .tools import Tool
from .resources import Resource
from .prompts import Prompt
from .response import McpResponse, ErrorCodes
from .schema import (
    python_type_to_json_schema,
    generate_function_input_schema,
    generate_function_output_schema,
    validate_against_schema
)
import os

__version__ = "0.1.0"

def get_wit_path():
    """Get the path to the bundled WIT file for componentize-py.
    
    Returns:
        Path to the mcp.wit file included with the SDK
        
    Example:
        >>> import wasmcp
        >>> wit_path = wasmcp.get_wit_path()
        >>> # Use with componentize-py:
        >>> # componentize-py -w {wit_path} -o handler.wasm my_handler.py
    """
    # Get the path to the wit directory relative to this file
    package_dir = os.path.dirname(os.path.abspath(__file__))
    wit_path = os.path.join(package_dir, '..', '..', 'wit', 'mcp.wit')
    # Normalize the path
    return os.path.normpath(wit_path)

def get_wit_world_path():
    """Get the path to the complete WIT world definition.
    
    Returns:
        Path to the world.wit file that includes all necessary imports
        
    Example:
        >>> import wasmcp
        >>> world_path = wasmcp.get_wit_world_path()
        >>> # Use for building components with full WASI support
    """
    package_dir = os.path.dirname(os.path.abspath(__file__))
    world_path = os.path.join(package_dir, '..', '..', 'wit', 'world.wit')
    return os.path.normpath(world_path)

def get_componentize_config():
    """Get the path to the componentize-py configuration.
    
    Returns:
        Path to the componentize-py.toml configuration file
    """
    package_dir = os.path.dirname(os.path.abspath(__file__))
    config_path = os.path.join(package_dir, 'componentize-py.toml')
    return os.path.normpath(config_path)

__all__ = [
    "WasmcpHandler",
    "Handler",
    "Tool",
    "Resource", 
    "Prompt",
    "McpResponse",
    "ErrorCodes",
    "python_type_to_json_schema",
    "generate_function_input_schema",
    "generate_function_output_schema",
    "validate_against_schema",
    "get_wit_path",
    "get_wit_world_path",
    "get_componentize_config",
    "setup_handler",
]

# Import the setup function for easy access
from .exports_adapter import setup_handler