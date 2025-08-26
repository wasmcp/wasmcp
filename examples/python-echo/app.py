"""Example echo handler for wasmcp."""

from wasmcp import WasmcpHandler

# Create handler instance
handler = WasmcpHandler("echo-handler")


@handler.tool
def echo(message: str) -> str:
    """Echo back the provided message.
    
    Args:
        message: The message to echo back
        
    Returns:
        The echoed message
    """
    return f"Echo: {message}"


@handler.tool
def reverse(text: str) -> str:
    """Reverse the provided text.
    
    Args:
        text: The text to reverse
        
    Returns:
        The reversed text
    """
    return text[::-1]


@handler.tool(name="shout", description="Convert text to uppercase")
def make_uppercase(text: str) -> str:
    """Convert text to uppercase.
    
    Args:
        text: The text to convert
        
    Returns:
        The uppercase text
    """
    return text.upper()


@handler.resource(uri="config://version")
def get_version() -> dict:
    """Get the handler version information."""
    return {
        "name": "echo-handler",
        "version": "1.0.0",
        "sdk": "wasmcp-python"
    }


@handler.resource(
    uri="data://capabilities",
    mime_type="application/json",
    description="Handler capabilities"
)
def get_capabilities() -> dict:
    """Get handler capabilities."""
    return {
        "tools": ["echo", "reverse", "shout"],
        "resources": ["config://version", "data://capabilities"],
        "features": ["text-manipulation", "configuration"]
    }


@handler.prompt
def greeting_prompt(name: str = "World") -> list:
    """Generate a greeting prompt.
    
    Args:
        name: Name to greet (default: World)
        
    Returns:
        List of prompt messages
    """
    return [
        {"role": "system", "content": "You are a friendly assistant."},
        {"role": "user", "content": f"Please greet {name} warmly."}
    ]


# Export the handler for WASM compilation
Handler = handler.build()