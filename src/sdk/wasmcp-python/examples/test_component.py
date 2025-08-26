"""Simple test component for validating WIT binding generation."""

from wasmcp import Handler

# Create handler instance
handler = Handler()

@handler.tool
def echo(message: str) -> str:
    """Echo the message back."""
    return f"Echo: {message}"

@handler.tool
def add(a: int, b: int) -> str:
    """Add two numbers."""
    return str(a + b)

# This ensures the handler is registered globally
print("Test component initialized with tools:", list(handler.tools.keys()))