"""JSON Schema generation from Python types."""

import inspect
from typing import Any, Dict, List, Optional, Union, get_origin, get_args
import sys


def python_type_to_json_schema(python_type: Any, required: bool = True) -> Dict[str, Any]:
    """Convert a Python type to JSON Schema.
    
    Args:
        python_type: Python type to convert
        required: Whether the field is required (affects Optional handling)
        
    Returns:
        JSON Schema representation
    """
    # Handle None type
    if python_type is type(None):
        return {"type": "null"}
    
    # Handle basic types
    if python_type is str:
        return {"type": "string"}
    elif python_type is int:
        return {"type": "integer"}
    elif python_type is float:
        return {"type": "number"}
    elif python_type is bool:
        return {"type": "boolean"}
    elif python_type is list:
        return {"type": "array"}
    elif python_type is dict:
        return {"type": "object"}
    
    # Handle typing generics
    origin = get_origin(python_type)
    args = get_args(python_type)
    
    if origin is Union:
        # Check if it's Optional (Union[T, None])
        if type(None) in args:
            # Filter out None and get the actual type
            non_none_args = [arg for arg in args if arg is not type(None)]
            if len(non_none_args) == 1:
                # It's Optional[T] - always nullable
                schema = python_type_to_json_schema(non_none_args[0], required=required)
                # Make it nullable
                if "type" in schema:
                    if isinstance(schema["type"], list):
                        schema["type"].append("null")
                    else:
                        schema["type"] = [schema["type"], "null"]
                else:
                    # For complex schemas without a simple type
                    return {"oneOf": [schema, {"type": "null"}]}
                return schema
        else:
            # Regular Union
            return {
                "oneOf": [python_type_to_json_schema(arg) for arg in args]
            }
    
    elif origin is list or origin is List:
        if args:
            return {
                "type": "array",
                "items": python_type_to_json_schema(args[0])
            }
        return {"type": "array"}
    
    elif origin is dict or origin is Dict:
        if len(args) >= 2:
            return {
                "type": "object",
                "additionalProperties": python_type_to_json_schema(args[1])
            }
        return {"type": "object"}
    
    # Handle Any
    if python_type is Any:
        return {}
    
    # For unknown types, return empty schema (allows anything)
    return {}


def generate_function_input_schema(func: callable) -> Dict[str, Any]:
    """Generate JSON Schema for function input parameters.
    
    Args:
        func: Function to analyze
        
    Returns:
        JSON Schema for function parameters
    """
    sig = inspect.signature(func)
    properties = {}
    required = []
    
    for param_name, param in sig.parameters.items():
        if param_name == 'self':
            continue
            
        # Get type annotation
        param_type = param.annotation if param.annotation != inspect.Parameter.empty else Any
        
        # Generate schema for this parameter
        param_schema = python_type_to_json_schema(param_type)
        properties[param_name] = param_schema
        
        # Check if required (no default value)
        if param.default is inspect.Parameter.empty:
            required.append(param_name)
    
    schema = {
        "type": "object",
        "properties": properties
    }
    
    if required:
        schema["required"] = required
        
    return schema


def generate_function_output_schema(func: callable) -> Dict[str, Any]:
    """Generate JSON Schema for function output.
    
    Args:
        func: Function to analyze
        
    Returns:
        JSON Schema for function return value
    """
    sig = inspect.signature(func)
    return_type = sig.return_annotation
    
    if return_type == inspect.Signature.empty:
        return_type = Any
    
    # Get the base schema
    schema = python_type_to_json_schema(return_type)
    
    # For primitive types, wrap in object with "result" key
    if schema.get("type") in ["string", "integer", "number", "boolean"]:
        return {
            "type": "object",
            "properties": {
                "result": schema
            },
            "required": ["result"]
        }
    
    # For complex types, return as-is
    return schema


def validate_against_schema(value: Any, schema: Dict[str, Any]) -> Optional[str]:
    """Validate a value against a JSON Schema.
    
    Args:
        value: Value to validate
        schema: JSON Schema to validate against
        
    Returns:
        Error message if validation fails, None if valid
    """
    def validate_type(val: Any, expected_type: Union[str, List[str]]) -> bool:
        """Check if value matches expected type(s)."""
        if isinstance(expected_type, list):
            return any(validate_type(val, t) for t in expected_type)
        
        type_map = {
            "string": str,
            "integer": int,
            "number": (int, float),
            "boolean": bool,
            "array": list,
            "object": dict,
            "null": type(None)
        }
        
        expected_python_type = type_map.get(expected_type)
        if expected_python_type is None:
            return True  # Unknown type, allow anything
            
        return isinstance(val, expected_python_type)
    
    # Handle empty schema (allows anything)
    if not schema:
        return None
    
    # Check type
    if "type" in schema:
        if not validate_type(value, schema["type"]):
            if isinstance(schema["type"], list):
                return f"Value {value} does not match any of the allowed types {schema['type']}"
            else:
                return f"Value {value} does not match expected type {schema['type']}"
    
    # Handle oneOf
    if "oneOf" in schema:
        for sub_schema in schema["oneOf"]:
            if validate_against_schema(value, sub_schema) is None:
                return None  # At least one schema matched
        return f"Value {value} does not match any of the oneOf schemas"
    
    # Validate array items
    if isinstance(value, list) and "items" in schema:
        for i, item in enumerate(value):
            error = validate_against_schema(item, schema["items"])
            if error:
                return f"Array item {i}: {error}"
    
    # Validate object properties
    if isinstance(value, dict):
        if "properties" in schema:
            for prop_name, prop_schema in schema["properties"].items():
                if prop_name in value:
                    error = validate_against_schema(value[prop_name], prop_schema)
                    if error:
                        return f"Property {prop_name}: {error}"
        
        if "required" in schema:
            for required_prop in schema["required"]:
                if required_prop not in value:
                    return f"Missing required property: {required_prop}"
    
    return None