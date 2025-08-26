"""Tests for prompt management module."""

import pytest
import json
from wasmcp.prompts import Prompt


class TestPrompt:
    """Test Prompt class."""
    
    def test_basic_prompt_creation(self):
        """Test creating a basic prompt."""
        def simple_prompt():
            """Simple prompt."""
            return [{"role": "user", "content": "Hello"}]
        
        prompt = Prompt(simple_prompt)
        assert prompt.name == "simple_prompt"
        assert prompt.description == "Simple prompt."
        assert "properties" in prompt.input_schema
    
    def test_prompt_with_custom_properties(self):
        """Test prompt with custom name and description."""
        def func():
            return []
        
        prompt = Prompt(
            func,
            name="custom_prompt",
            description="Custom description"
        )
        assert prompt.name == "custom_prompt"
        assert prompt.description == "Custom description"
    
    def test_from_function_classmethod(self):
        """Test creating prompt from function using classmethod."""
        def test_prompt():
            return [{"role": "system", "content": "Test"}]
        
        prompt = Prompt.from_function(test_prompt)
        assert prompt.name == "test_prompt"
        assert isinstance(prompt.input_schema, dict)
    
    def test_to_dict_no_arguments(self):
        """Test converting prompt with no arguments to dictionary."""
        def no_args_prompt():
            """No arguments prompt."""
            return [{"role": "user", "content": "Test"}]
        
        prompt = Prompt(no_args_prompt)
        result = prompt.to_dict()
        
        expected = {
            "name": "no_args_prompt",
            "description": "No arguments prompt."
        }
        assert result == expected
    
    def test_to_dict_with_arguments(self):
        """Test converting prompt with arguments to dictionary."""
        def prompt_with_args(topic: str, style: str = "formal"):
            return [{"role": "system", "content": f"Write about {topic} in {style} style"}]
        
        prompt = Prompt(prompt_with_args)
        result = prompt.to_dict()
        
        assert result["name"] == "prompt_with_args"
        assert "arguments" in result
        
        args = result["arguments"]
        assert len(args) == 2
        
        # Find topic and style arguments
        topic_arg = next(arg for arg in args if arg["name"] == "topic")
        style_arg = next(arg for arg in args if arg["name"] == "style")
        
        assert topic_arg["required"] is True
        assert topic_arg["type"] == "string"
        assert style_arg["required"] is False
        assert style_arg["type"] == "string"
    
    def test_get_prompt_no_args(self):
        """Test generating prompt with no arguments."""
        def simple_prompt():
            return [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello"}
            ]
        
        prompt = Prompt(simple_prompt)
        result = prompt.get_prompt('{}')
        
        assert "result" in result
        messages = result["result"]["messages"]
        assert len(messages) == 2
        
        assert messages[0]["role"] == "system"
        assert messages[0]["content"]["type"] == "text"
        assert messages[0]["content"]["text"] == "You are helpful."
        
        assert messages[1]["role"] == "user"
        assert messages[1]["content"]["text"] == "Hello"
    
    def test_get_prompt_with_args(self):
        """Test generating prompt with arguments."""
        def greeting_prompt(name: str = "World"):
            return [{"role": "user", "content": f"Hello, {name}!"}]
        
        prompt = Prompt(greeting_prompt)
        result = prompt.get_prompt('{"name": "Alice"}')
        
        messages = result["result"]["messages"]
        assert len(messages) == 1
        assert messages[0]["content"]["text"] == "Hello, Alice!"
    
    def test_get_prompt_with_dict_args(self):
        """Test generating prompt with dict arguments."""
        def custom_prompt(topic: str):
            return [{"role": "system", "content": f"Talk about {topic}"}]
        
        prompt = Prompt(custom_prompt)
        result = prompt.get_prompt({"topic": "science"})
        
        messages = result["result"]["messages"]
        assert messages[0]["content"]["text"] == "Talk about science"
    
    def test_get_prompt_invalid_json(self):
        """Test generating prompt with invalid JSON."""
        def dummy_prompt():
            return []
        
        prompt = Prompt(dummy_prompt)
        result = prompt.get_prompt('{"invalid": json}')
        
        assert "error" in result
        assert result["error"]["code"] == -32602
    
    def test_get_prompt_missing_required_arg(self):
        """Test generating prompt with missing required argument."""
        def requires_arg(name: str):
            return [{"role": "user", "content": f"Hello {name}"}]
        
        prompt = Prompt(requires_arg)
        result = prompt.get_prompt('{}')
        
        assert "error" in result
        assert result["error"]["code"] == -32602
    
    def test_get_prompt_function_error(self):
        """Test generating prompt when function raises an error."""
        def failing_prompt():
            raise ValueError("Prompt generation failed")
        
        prompt = Prompt(failing_prompt)
        result = prompt.get_prompt('{}')
        
        assert "error" in result
        assert result["error"]["code"] == -32603
        assert "Prompt generation failed" in result["error"]["message"]
    
    def test_get_prompt_invalid_return(self):
        """Test prompt function that doesn't return a list."""
        def invalid_prompt():
            return "not a list"
        
        prompt = Prompt(invalid_prompt)
        result = prompt.get_prompt('{}')
        
        assert "error" in result
        assert "must return a list" in result["error"]["message"]
    
    def test_get_prompt_invalid_message_format(self):
        """Test prompt function that returns invalid message format."""
        def invalid_message_prompt():
            return [{"invalid": "format"}]
        
        prompt = Prompt(invalid_message_prompt)
        result = prompt.get_prompt('{}')
        
        assert "error" in result
        assert "Invalid message format" in result["error"]["message"]