"""Integration tests for wasmcp Python SDK.

These tests verify that all components work together correctly.
"""

import pytest
import json
from unittest.mock import patch, Mock
from wasmcp import WasmcpHandler
from wasmcp.exports import WasmcpExports


class TestIntegration:
    """Integration tests for complete handler functionality."""
    
    def test_complete_handler_workflow(self):
        """Test a complete handler workflow with all component types."""
        # Create handler
        handler = WasmcpHandler("integration-test")
        
        # Register tools
        @handler.tool
        def calculate(operation: str, a: float, b: float) -> float:
            """Perform calculation."""
            if operation == "add":
                return a + b
            elif operation == "multiply":
                return a * b
            elif operation == "divide":
                if b == 0:
                    raise ValueError("Division by zero")
                return a / b
            else:
                raise ValueError(f"Unknown operation: {operation}")
        
        @handler.tool(name="format_number")
        def fmt_num(value: float, decimals: int = 2) -> str:
            """Format a number."""
            return f"{value:.{decimals}f}"
        
        # Register resources
        @handler.resource(uri="config://calculator", mime_type="application/json")
        def get_config() -> dict:
            """Get calculator configuration."""
            return {
                "version": "1.0.0",
                "operations": ["add", "multiply", "divide"],
                "precision": 10
            }
        
        @handler.resource(uri="data://history")
        def get_history() -> list:
            """Get calculation history."""
            return [
                {"operation": "add", "a": 1, "b": 2, "result": 3},
                {"operation": "multiply", "a": 5, "b": 4, "result": 20}
            ]
        
        # Register prompts
        @handler.prompt
        def math_tutor(topic: str = "arithmetic") -> list:
            """Generate math tutoring prompt."""
            return [
                {"role": "system", "content": f"You are a helpful {topic} tutor."},
                {"role": "user", "content": "Help me understand this problem."}
            ]
        
        # Build handler to set up exports
        handler.build()
        
        # Create exports for testing
        exports = WasmcpExports(handler)
        
        # Test listing tools
        tools = exports.list_tools()
        assert len(tools) == 2
        tool_names = [t["name"] for t in tools]
        assert "calculate" in tool_names
        assert "format_number" in tool_names
        
        # Test calling tools
        calc_result = exports.call_tool("calculate", json.dumps({
            "operation": "add",
            "a": 10,
            "b": 20
        }))
        assert calc_result["result"]["text"] == "30"
        
        format_result = exports.call_tool("format_number", json.dumps({
            "value": 3.14159,
            "decimals": 3
        }))
        assert format_result["result"]["text"] == "3.142"
        
        # Test tool error handling
        error_result = exports.call_tool("calculate", json.dumps({
            "operation": "divide",
            "a": 10,
            "b": 0
        }))
        assert "error" in error_result
        assert "Division by zero" in error_result["error"]["message"]
        
        # Test listing resources
        resources = exports.list_resources()
        assert len(resources) == 2
        resource_uris = [r["uri"] for r in resources]
        assert "config://calculator" in resource_uris
        assert "data://history" in resource_uris
        
        # Test reading resources
        config_result = exports.read_resource("config://calculator")
        assert "result" in config_result
        config_data = json.loads(config_result["result"]["contents"][0]["text"])
        assert config_data["version"] == "1.0.0"
        assert len(config_data["operations"]) == 3
        
        history_result = exports.read_resource("data://history")
        assert "result" in history_result
        history_data = json.loads(history_result["result"]["contents"][0]["text"])
        assert len(history_data) == 2
        assert history_data[0]["operation"] == "add"
        
        # Test listing prompts
        prompts = exports.list_prompts()
        assert len(prompts) == 1
        assert prompts[0]["name"] == "math_tutor"
        assert len(prompts[0]["arguments"]) == 1
        assert prompts[0]["arguments"][0]["name"] == "topic"
        
        # Test getting prompts
        prompt_result = exports.get_prompt("math_tutor", json.dumps({
            "topic": "calculus"
        }))
        assert "result" in prompt_result
        assert len(prompt_result["result"]["messages"]) == 2
        assert "calculus tutor" in prompt_result["result"]["messages"][0]["content"]["text"]
    
    def test_wit_export_functions(self):
        """Test the module-level WIT export functions."""
        import wasmcp.exports as exports_module
        
        # Create and build handler
        handler = WasmcpHandler("export-test")
        
        @handler.tool
        def test_tool(input: str) -> str:
            return f"Processed: {input}"
        
        @handler.resource(uri="test://data")
        def test_resource() -> dict:
            return {"test": "data"}
        
        @handler.prompt
        def test_prompt() -> list:
            return [{"role": "user", "content": "Test"}]
        
        handler.build()
        
        # Test module-level functions
        tools_json = exports_module.list_tools()
        tools = json.loads(tools_json)
        assert len(tools) == 1
        assert tools[0]["name"] == "test_tool"
        
        result_json = exports_module.call_tool("test_tool", json.dumps({"input": "hello"}))
        result = json.loads(result_json)
        assert result["result"]["text"] == "Processed: hello"
        
        resources_json = exports_module.list_resources()
        resources = json.loads(resources_json)
        assert len(resources) == 1
        assert resources[0]["uri"] == "test://data"
        
        resource_json = exports_module.read_resource("test://data")
        resource = json.loads(resource_json)
        assert json.loads(resource["result"]["contents"][0]["text"]) == {"test": "data"}
        
        prompts_json = exports_module.list_prompts()
        prompts = json.loads(prompts_json)
        assert len(prompts) == 1
        assert prompts[0]["name"] == "test_prompt"
        
        prompt_json = exports_module.get_prompt("test_prompt", "{}")
        prompt = json.loads(prompt_json)
        assert len(prompt["result"]["messages"]) == 1
        assert prompt["result"]["messages"][0]["content"]["text"] == "Test"
    
    def test_async_components(self):
        """Test handler with async components."""
        handler = WasmcpHandler("async-test")
        
        # Async tool
        async def async_process(data: str) -> str:
            # In real WASM, this would use actual async operations
            return data.upper()
        
        from wasmcp.tools import Tool
        handler._tools["async_process"] = Tool(async_process)
        
        # Async resource
        async def async_fetch() -> dict:
            # In real WASM, this would fetch async data
            return {"async": True}
        
        from wasmcp.resources import Resource
        handler._resources["async://data"] = Resource(
            async_fetch,
            uri="async://data"
        )
        
        # Async prompt
        async def async_prompt() -> list:
            return [{"role": "user", "content": "Async"}]
        
        from wasmcp.prompts import Prompt
        handler._prompts["async_prompt"] = Prompt(async_prompt)
        
        exports = WasmcpExports(handler)
        
        # Test async tool execution
        result = exports.call_tool("async_process", json.dumps({"data": "hello"}))
        assert result["result"]["text"] == "HELLO"
        
        # Test async resource reading
        result = exports.read_resource("async://data")
        assert json.loads(result["result"]["contents"][0]["text"]) == {"async": True}
        
        # Test async prompt generation
        result = exports.get_prompt("async_prompt", "{}")
        assert result["result"]["messages"][0]["content"]["text"] == "Async"
    
    def test_error_scenarios(self):
        """Test various error scenarios."""
        handler = WasmcpHandler("error-test")
        exports = WasmcpExports(handler)
        
        # Test calling non-existent tool
        result = exports.call_tool("nonexistent", "{}")
        assert result["error"]["code"] == -32601
        
        # Test reading non-existent resource
        result = exports.read_resource("nonexistent://uri")
        assert result["error"]["code"] == -32601
        
        # Test getting non-existent prompt
        result = exports.get_prompt("nonexistent", "{}")
        assert result["error"]["code"] == -32601
        
        # Add components that fail
        @handler.tool
        def failing_tool() -> str:
            raise RuntimeError("Tool failure")
        
        @handler.resource(uri="fail://resource")
        def failing_resource():
            raise RuntimeError("Resource failure")
        
        @handler.prompt
        def failing_prompt() -> list:
            raise RuntimeError("Prompt failure")
        
        exports = WasmcpExports(handler)
        
        # Test execution failures
        result = exports.call_tool("failing_tool", "{}")
        assert result["error"]["code"] == -32603
        assert "Tool failure" in result["error"]["message"]
        
        result = exports.read_resource("fail://resource")
        assert result["error"]["code"] == -32603
        assert "Resource failure" in result["error"]["message"]
        
        result = exports.get_prompt("failing_prompt", "{}")
        assert result["error"]["code"] == -32603
        assert "Prompt failure" in result["error"]["message"]