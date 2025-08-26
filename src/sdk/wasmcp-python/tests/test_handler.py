"""Tests for handler module."""

import pytest
from wasmcp.handler import WasmcpHandler
from wasmcp.tools import Tool
from wasmcp.resources import Resource
from wasmcp.prompts import Prompt


class TestWasmcpHandler:
    """Test WasmcpHandler class."""
    
    def test_basic_handler_creation(self):
        """Test creating a basic handler."""
        handler = WasmcpHandler("test-handler")
        assert handler.name == "test-handler"
        assert len(handler._tools) == 0
        assert len(handler._resources) == 0
        assert len(handler._prompts) == 0
    
    def test_default_handler_name(self):
        """Test handler with default name."""
        handler = WasmcpHandler()
        assert handler.name == "wasmcp-python-handler"
    
    def test_tool_decorator_basic(self):
        """Test basic tool decorator usage."""
        handler = WasmcpHandler("test")
        
        @handler.tool
        def test_function(param: str) -> str:
            """Test function."""
            return f"Result: {param}"
        
        assert len(handler._tools) == 1
        assert "test_function" in handler._tools
        assert isinstance(handler._tools["test_function"], Tool)
        assert handler._tools["test_function"].name == "test_function"
    
    def test_tool_decorator_with_options(self):
        """Test tool decorator with custom options."""
        handler = WasmcpHandler("test")
        
        @handler.tool(name="custom_name", description="Custom description")
        def test_function() -> str:
            return "test"
        
        tool = handler._tools["custom_name"]
        assert tool.name == "custom_name"
        assert tool.description == "Custom description"
    
    def test_resource_decorator_basic(self):
        """Test basic resource decorator usage."""
        handler = WasmcpHandler("test")
        
        @handler.resource(uri="config://test")
        def get_config() -> dict:
            """Get configuration."""
            return {"version": "1.0"}
        
        assert len(handler._resources) == 1
        assert "config://test" in handler._resources
        assert isinstance(handler._resources["config://test"], Resource)
    
    def test_resource_decorator_with_options(self):
        """Test resource decorator with custom options."""
        handler = WasmcpHandler("test")
        
        @handler.resource(
            uri="data://users",
            name="User Data",
            mime_type="application/json"
        )
        def get_users() -> list:
            return [{"id": 1, "name": "Alice"}]
        
        resource = handler._resources["data://users"]
        assert resource.name == "User Data"
        assert resource.mime_type == "application/json"
    
    def test_resource_decorator_missing_uri(self):
        """Test resource decorator without required URI."""
        handler = WasmcpHandler("test")
        
        with pytest.raises(ValueError, match="Resource decorator requires 'uri' parameter"):
            @handler.resource()
            def invalid_resource():
                return {}
    
    def test_prompt_decorator_basic(self):
        """Test basic prompt decorator usage."""
        handler = WasmcpHandler("test")
        
        @handler.prompt
        def test_prompt() -> list:
            """Test prompt."""
            return [{"role": "user", "content": "Test"}]
        
        assert len(handler._prompts) == 1
        assert "test_prompt" in handler._prompts
        assert isinstance(handler._prompts["test_prompt"], Prompt)
    
    def test_prompt_decorator_with_options(self):
        """Test prompt decorator with custom options."""
        handler = WasmcpHandler("test")
        
        @handler.prompt(name="custom_prompt", description="Custom prompt")
        def test_prompt() -> list:
            return [{"role": "system", "content": "Test"}]
        
        prompt = handler._prompts["custom_prompt"]
        assert prompt.name == "custom_prompt"
        assert prompt.description == "Custom prompt"
    
    def test_multiple_decorators(self):
        """Test using multiple types of decorators."""
        handler = WasmcpHandler("multi-test")
        
        @handler.tool
        def tool_func(x: int) -> int:
            return x * 2
        
        @handler.resource(uri="config://settings")
        def resource_func() -> dict:
            return {"setting": "value"}
        
        @handler.prompt
        def prompt_func() -> list:
            return [{"role": "user", "content": "Hello"}]
        
        assert len(handler._tools) == 1
        assert len(handler._resources) == 1
        assert len(handler._prompts) == 1
    
    def test_build_method(self):
        """Test the build method."""
        handler = WasmcpHandler("build-test")
        
        @handler.tool
        def test_tool():
            return "test"
        
        result = handler.build()
        assert result is handler
        # Should still have the registered tool
        assert len(handler._tools) == 1
    
    def test_repr(self):
        """Test string representation."""
        handler = WasmcpHandler("repr-test")
        
        @handler.tool
        def tool1():
            return "1"
        
        @handler.tool  
        def tool2():
            return "2"
        
        @handler.resource(uri="test://resource")
        def resource1():
            return {}
        
        repr_str = repr(handler)
        assert "repr-test" in repr_str
        assert "tools=2" in repr_str
        assert "resources=1" in repr_str
        assert "prompts=0" in repr_str