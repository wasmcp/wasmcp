"""Tests for WIT exports module."""

import pytest
import json
from unittest.mock import Mock
from wasmcp.exports import WasmcpExports
from wasmcp.handler import WasmcpHandler


class TestWasmcpExports:
    """Test WasmcpExports class."""
    
    def setup_method(self):
        """Set up test handler and exports."""
        self.handler = WasmcpHandler("test-handler")
        self.exports = WasmcpExports(self.handler)
    
    def test_list_tools_empty(self):
        """Test listing tools when no tools are registered."""
        result = self.exports.list_tools()
        assert result == []
    
    def test_list_tools_with_tools(self):
        """Test listing tools when tools are registered."""
        @self.handler.tool
        def test_tool(param: str) -> str:
            """Test tool."""
            return param
        
        result = self.exports.list_tools()
        assert len(result) == 1
        assert result[0]["name"] == "test_tool"
        assert result[0]["description"] == "Test tool."
        assert "inputSchema" in result[0]
    
    def test_call_tool_success(self):
        """Test successful tool call."""
        @self.handler.tool
        def add(a: int, b: int) -> int:
            return a + b
        
        result = self.exports.call_tool("add", '{"a": 5, "b": 3}')
        assert "result" in result
        assert result["result"]["text"] == "8"
    
    def test_call_tool_not_found(self):
        """Test calling non-existent tool."""
        result = self.exports.call_tool("nonexistent", '{}')
        assert "error" in result
        assert result["error"]["code"] == -32601
    
    def test_list_resources_empty(self):
        """Test listing resources when no resources are registered."""
        result = self.exports.list_resources()
        assert result == []
    
    def test_list_resources_with_resources(self):
        """Test listing resources when resources are registered."""
        @self.handler.resource(uri="config://test")
        def test_resource() -> dict:
            """Test resource."""
            return {"test": True}
        
        result = self.exports.list_resources()
        assert len(result) == 1
        assert result[0]["uri"] == "config://test"
        assert result[0]["name"] == "test_resource"
        assert result[0]["description"] == "Test resource."
    
    def test_read_resource_success(self):
        """Test successful resource read."""
        @self.handler.resource(uri="data://test")
        def test_data():
            return "test content"
        
        result = self.exports.read_resource("data://test")
        assert "result" in result
        contents = result["result"]["contents"][0]
        assert contents["uri"] == "data://test"
        assert contents["text"] == "test content"
    
    def test_read_resource_not_found(self):
        """Test reading non-existent resource."""
        result = self.exports.read_resource("nonexistent://uri")
        assert "error" in result
        assert result["error"]["code"] == -32601
    
    def test_list_prompts_empty(self):
        """Test listing prompts when no prompts are registered."""
        result = self.exports.list_prompts()
        assert result == []
    
    def test_list_prompts_with_prompts(self):
        """Test listing prompts when prompts are registered."""
        @self.handler.prompt
        def test_prompt() -> list:
            """Test prompt."""
            return [{"role": "user", "content": "Test"}]
        
        result = self.exports.list_prompts()
        assert len(result) == 1
        assert result[0]["name"] == "test_prompt"
        assert result[0]["description"] == "Test prompt."
    
    def test_get_prompt_success(self):
        """Test successful prompt generation."""
        @self.handler.prompt
        def greeting(name: str = "World") -> list:
            return [{"role": "user", "content": f"Hello, {name}!"}]
        
        result = self.exports.get_prompt("greeting", '{"name": "Alice"}')
        assert "result" in result
        messages = result["result"]["messages"]
        assert len(messages) == 1
        assert messages[0]["content"]["text"] == "Hello, Alice!"
    
    def test_get_prompt_not_found(self):
        """Test getting non-existent prompt."""
        result = self.exports.get_prompt("nonexistent", '{}')
        assert "error" in result
        assert result["error"]["code"] == -32601


class TestWitExportFunctions:
    """Test module-level WIT export functions."""
    
    def test_exports_without_handler(self):
        """Test export functions when no handler is initialized."""
        import wasmcp.exports as exports_module
        
        # Temporarily clear the global exports
        original_exports = exports_module._exports
        exports_module._exports = None
        
        try:
            # Test each export function
            assert json.loads(exports_module.list_tools()) == []
            assert json.loads(exports_module.list_resources()) == []
            assert json.loads(exports_module.list_prompts()) == []
            
            # Error cases should return internal error
            result = json.loads(exports_module.call_tool("test", "{}"))
            assert result["error"]["code"] == -32603
            
            result = json.loads(exports_module.read_resource("test://uri"))
            assert result["error"]["code"] == -32603
            
            result = json.loads(exports_module.get_prompt("test", "{}"))
            assert result["error"]["code"] == -32603
            
        finally:
            # Restore original exports
            exports_module._exports = original_exports