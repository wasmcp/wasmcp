"""Tests for resource management module."""

import pytest
import json
from wasmcp.resources import Resource


class TestResource:
    """Test Resource class."""
    
    def test_basic_resource_creation(self):
        """Test creating a basic resource."""
        def get_data():
            """Get some data."""
            return "test data"
        
        resource = Resource(get_data, "test://data")
        assert resource.uri == "test://data"
        assert resource.name == "get_data"
        assert resource.description == "Get some data."
        assert resource.mime_type == "text/plain"
    
    def test_resource_with_custom_properties(self):
        """Test resource with custom name, description, and mime type."""
        def func():
            return {}
        
        resource = Resource(
            func,
            "config://settings",
            name="Settings",
            description="Application settings",
            mime_type="application/json"
        )
        assert resource.name == "Settings"
        assert resource.description == "Application settings"
        assert resource.mime_type == "application/json"
    
    def test_from_function_classmethod(self):
        """Test creating resource from function using classmethod."""
        def data_func():
            return {"test": True}
        
        resource = Resource.from_function(
            data_func,
            "data://test",
            mime_type="application/json"
        )
        assert resource.uri == "data://test"
        assert resource.mime_type == "application/json"
    
    def test_to_dict(self):
        """Test converting resource to dictionary."""
        def example_resource():
            """Example resource."""
            return "data"
        
        resource = Resource(
            example_resource,
            "example://resource",
            description="Test resource",
            mime_type="text/plain"
        )
        result = resource.to_dict()
        
        expected = {
            "uri": "example://resource",
            "name": "example_resource",
            "description": "Test resource",
            "mimeType": "text/plain"
        }
        assert result == expected
    
    def test_to_dict_minimal(self):
        """Test to_dict with minimal properties."""
        def minimal():
            return "data"
        
        resource = Resource(minimal, "test://minimal")
        result = resource.to_dict()
        
        # Should only include uri, name, and mimeType
        assert result["uri"] == "test://minimal"
        assert result["name"] == "minimal" 
        assert result["mimeType"] == "text/plain"
    
    def test_read_string_data(self):
        """Test reading string data."""
        def get_string():
            return "Hello, World!"
        
        resource = Resource(get_string, "string://data")
        result = resource.read()
        
        assert "result" in result
        contents = result["result"]["contents"][0]
        assert contents["uri"] == "string://data"
        assert contents["mimeType"] == "text/plain"
        assert contents["text"] == "Hello, World!"
    
    def test_read_json_data(self):
        """Test reading JSON data."""
        def get_json():
            return {"message": "Hello", "count": 42}
        
        resource = Resource(get_json, "json://data", mime_type="application/json")
        result = resource.read()
        
        contents = result["result"]["contents"][0]
        assert contents["mimeType"] == "application/json"
        
        # Should be formatted JSON
        data = json.loads(contents["text"])
        assert data == {"message": "Hello", "count": 42}
    
    def test_read_dict_as_text(self):
        """Test reading dict data with text mime type."""
        def get_dict():
            return {"key": "value"}
        
        resource = Resource(get_dict, "dict://data", mime_type="text/plain")
        result = resource.read()
        
        contents = result["result"]["contents"][0]
        # Should be JSON formatted even with text mime type
        data = json.loads(contents["text"])
        assert data == {"key": "value"}
    
    def test_read_with_function_error(self):
        """Test reading resource when function raises an error."""
        def failing_resource():
            raise RuntimeError("Resource unavailable")
        
        resource = Resource(failing_resource, "fail://resource")
        result = resource.read()
        
        assert "error" in result
        assert result["error"]["code"] == -32603
        assert "Resource unavailable" in result["error"]["message"]