"""Wrapper module to expose WIT exports at top level."""

# Import and set up the handler
from src.app import handler
from wasmcp.wit.exports import set_handler

# Register the handler
set_handler(handler)

# Import the WIT exports at module level - these are what componentize-py looks for
from wasmcp.wit.exports import (
    list_tools, call_tool, list_resources, 
    read_resource, list_prompts, get_prompt
)

# Also expose Handler class if needed
from wasmcp import Handler