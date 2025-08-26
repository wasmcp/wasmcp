"""Tests for tool management module."""

import pytest
import json
from unittest.mock import Mock
from wasmcp.tools import Tool


class TestTool:
    """Test Tool class."""
    
    def test_basic_tool_creation(self):
        """Test creating a basic tool."""
        def simple_func(name: str) -> str:
            """Simple function."""
            return f"Hello, {name}!"
        
        tool = Tool(simple_func)
        assert tool.name == "simple_func"
        assert tool.description == "Simple function."
        assert "properties" in tool.input_schema
    
    def test_tool_with_custom_name_and_description(self):
        """Test tool with custom name and description."""
        def func():
            pass
        
        tool = Tool(func, name="custom_name", description="Custom description")
        assert tool.name == "custom_name"
        assert tool.description == "Custom description"
    
    def test_from_function_classmethod(self):
        """Test creating tool from function using classmethod."""
        def test_func(x: int) -> int:
            return x * 2
        
        tool = Tool.from_function(test_func)
        assert tool.name == "test_func"
        assert isinstance(tool.input_schema, dict)
    
    def test_to_dict(self):
        """Test converting tool to dictionary."""
        def example_func(param: str) -> str:
            """Example function."""
            return param
        
        tool = Tool(example_func)
        result = tool.to_dict()
        
        expected = {
            "name": "example_func",
            "description": "Example function.",
            "inputSchema": tool.input_schema
        }
        assert result == expected
    
    def test_call_with_valid_args(self):
        """Test calling tool with valid arguments."""
        def add(a: int, b: int) -> int:
            return a + b
        
        tool = Tool(add)
        result = tool.call('{"a": 5, "b": 3}')
        
        assert "result" in result
        assert result["result"]["text"] == "8"
    
    def test_call_with_dict_args(self):
        """Test calling tool with dict arguments."""
        def greet(name: str) -> str:
            return f"Hello, {name}!"
        
        tool = Tool(greet)
        result = tool.call({"name": "Alice"})
        
        assert "result" in result
        assert result["result"]["text"] == "Hello, Alice!"
    
    def test_call_with_invalid_json(self):
        """Test calling tool with invalid JSON."""
        def dummy():
            pass
        
        tool = Tool(dummy)
        result = tool.call('{"invalid": json}')
        
        assert "error" in result
        assert result["error"]["code"] == -32602
    
    def test_call_with_missing_args(self):
        """Test calling tool with missing required arguments."""
        def requires_arg(name: str) -> str:
            return name
        
        tool = Tool(requires_arg)
        result = tool.call('{}')
        
        assert "error" in result
        assert result["error"]["code"] == -32602
    
    def test_call_with_function_error(self):
        """Test calling tool that raises an exception."""
        def failing_func():
            raise ValueError("Test error")
        
        tool = Tool(failing_func)
        result = tool.call('{}')
        
        assert "error" in result
        assert result["error"]["code"] == -32603
        assert "Test error" in result["error"]["message"]
    
    def test_call_with_complex_return_type(self):
        """Test tool that returns complex data."""
        def get_data() -> dict:
            return {"key": "value", "number": 42}
        
        tool = Tool(get_data)
        result = tool.call('{}')
        
        assert "result" in result
        response_data = json.loads(result["result"]["text"])
        assert response_data == {"key": "value", "number": 42}