"""MCP handler entry point for componentize-py."""

# Import the user's handler
from src.app import handler

# Set up the handler for WIT exports
from wasmcp import setup_handler
setup_handler(handler)

# Import the Handler class that componentize-py expects
from wasmcp.exports_adapter import Handler