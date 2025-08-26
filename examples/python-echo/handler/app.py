"""MCP handler entry point for componentize-py."""

# Import the exports module which implements the WIT interface
import exports

# Re-export the handler functions at module level for componentize-py
list_tools = exports.list_tools
call_tool = exports.call_tool
list_resources = exports.list_resources
read_resource = exports.read_resource
list_prompts = exports.list_prompts
get_prompt = exports.get_prompt

# Componentize-py expects a Handler class with these methods
class Handler:
    def list_tools(self):
        return exports.list_tools()
    
    def call_tool(self, name, arguments):
        return exports.call_tool(name, arguments)
    
    def list_resources(self):
        return exports.list_resources()
    
    def read_resource(self, uri):
        return exports.read_resource(uri)
    
    def list_prompts(self):
        return exports.list_prompts()
    
    def get_prompt(self, name, arguments):
        return exports.get_prompt(name, arguments)